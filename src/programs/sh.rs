use crate::{
    process::Process,
    programs::common::{
        extendable_iterator::ExtendableIterator,
        readline::{FileBasedHistory, Readline},
        shell_commands,
    },
    streams,
};
use anyhow::{anyhow, bail, Result};
use clap::Parser;
use futures::{
    channel::oneshot,
    future::{BoxFuture, FutureExt},
    io::{AsyncReadExt, AsyncWriteExt},
    join, select,
    stream::{AbortHandle, Abortable},
    try_join,
};
use std::future::Future;
use vfs::VfsPath;

const HISTORY_FILE: &str = "/etc/.sh_history";

#[derive(Debug, PartialEq, Eq)]
enum BasicToken {
    Pipe,
    FileRedirectOut { append: bool },
    FileRedirectIn,
    Value(String),
}

#[derive(PartialEq, Eq)]
enum QuoteType {
    None,
    Single,
    Double,
}

enum Token {
    Pipe(Box<Token>, Box<Token>),
    FileRedirectOut {
        lhs: Box<Token>,
        append: bool,
        path: String,
    },
    FileRedirectIn {
        lhs: Box<Token>,
        path: String,
    },
    Command(Vec<String>),
}

fn parse(basic_tokens: Vec<BasicToken>) -> Result<Token> {
    let mut root = Token::Command(Vec::new());

    for basic in basic_tokens {
        match basic {
            BasicToken::Value(value) => match &mut root {
                Token::Pipe(_, subtoken) => match &mut **subtoken {
                    Token::Command(values) => {
                        values.push(value);
                    }
                    _ => unreachable!("Pipes cannot be nested on rhs"),
                },
                Token::FileRedirectOut {
                    lhs,
                    append: _,
                    path,
                }
                | Token::FileRedirectIn { lhs, path } => {
                    if path.is_empty() {
                        *path = value;
                    } else {
                        match &mut **lhs {
                            Token::Command(values) => values.push(value),
                            _ => bail!("Syntax error: file redirect has non-command token as lhs"),
                        }
                    }
                }
                Token::Command(values) => values.push(value),
            },
            BasicToken::Pipe => {
                root = Token::Pipe(Box::new(root), Box::new(Token::Command(Vec::new())));
            }
            BasicToken::FileRedirectOut { append } => {
                root = Token::FileRedirectOut {
                    lhs: Box::new(root),
                    append,
                    path: String::new(),
                };
            }
            BasicToken::FileRedirectIn => {
                root = Token::FileRedirectIn {
                    lhs: Box::new(root),
                    path: String::new(),
                };
            }
        }
    }

    Ok(root)
}

// Parses a variable or substring, injects the value into the source iterator, and returns the
// length of the value.
async fn parse_variable(process: &Process, source: &mut ExtendableIterator<char>) -> Result<usize> {
    let Some(delimiter) = source.next() else {
        bail!("Syntax error: No identifier");
    };
    if delimiter != '{' && delimiter != '(' {
        bail!("Currently only ${{...}} variables and subshells are supported");
    }

    let mut value = String::new();
    while let Some(c) = source.next() {
        // Subshell
        if delimiter == '(' && c == ')' {
            let (mut reader, mut stdout, mut backend) = streams::pipe();

            let mut process = process.clone();
            process.stdout = stdout.clone();

            let (abort_channel_tx, abort_channel_rx) = oneshot::channel();
            process.signal_registrar.unbounded_send(abort_channel_tx)?;

            let future = async {
                let (_, output): (Result<()>, Result<String>) = join! {
                    backend.run(),
                    async {
                        let mut output = String::new();
                        run_script(&mut process, &value).await?;
                        stdout.flush().await?;
                        stdout.shutdown().await?;
                        reader.read_to_string(&mut output).await?;
                        reader.shutdown().await?;
                        Ok(output)
                    },
                };
                output
            };
            let output = await_abortable_future(abort_channel_rx, future).await?;

            source.prepend(output.chars());
            return Ok(output.len());
        // Variables
        } else if delimiter == '{' && c == '}' {
            let value = if let Some(value) = process.env.get(&value) {
                value.to_string()
            // Display all args (except the first)
            } else if value == "@" {
                let args: Vec<String> = process.args.iter().skip(1).cloned().collect();
                args.join(" ")
            // Display an argument
            } else if let Ok(value) = value.parse::<u8>() {
                if let Some(value) = process.args.get(value as usize) {
                    value.to_string()
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            source.prepend(value.chars());
            return Ok(value.len());
        } else {
            value.push(c);
        }
    }

    Err(anyhow!("Syntax error: brace mismatch"))
}

async fn tokenize(process: &mut Process, source: &str) -> Result<Vec<BasicToken>> {
    let mut quote_level = QuoteType::None;
    let mut tokens = Vec::new();
    let mut buffer = String::new();
    let mut source = ExtendableIterator::new(source.chars());
    // We don't consider quote changes or start of variables when we're inside the result of a
    // variable/subshell
    let mut ignore_quotes: usize = 0;
    // Current character
    let mut c = None;

    loop {
        let last_char = c;
        c = source.next();
        let Some(c) = c else {break;};
        ignore_quotes = ignore_quotes.saturating_sub(1);

        if quote_level == QuoteType::None && [' ', '\n', '\t'].contains(&c) {
            if !buffer.is_empty() {
                tokens.push(BasicToken::Value(buffer.clone()));
                buffer.clear();
            }
            continue;
        }

        if ignore_quotes == 0 {
            match quote_level {
                QuoteType::None => {
                    if c == '\'' {
                        quote_level = QuoteType::Single;
                        continue;
                    } else if c == '"' {
                        quote_level = QuoteType::Double;
                        continue;
                    } else if c == '#' {
                        break;
                    } else if c == '$' {
                        // Extra 1 because we sub on beginning of loop
                        ignore_quotes = 1 + parse_variable(process, &mut source).await?;
                        continue;
                    } else if ['|', '>', '<'].contains(&c) {
                        if !buffer.is_empty() {
                            tokens.push(BasicToken::Value(buffer.clone()));
                            buffer.clear();
                        }

                        if c == '|' {
                            tokens.push(BasicToken::Pipe);
                        } else if c == '>' {
                            if last_char == Some('>') {
                                if !matches!(
                                    tokens.pop(),
                                    Some(BasicToken::FileRedirectOut { append: false })
                                ) {
                                    bail!("Syntax error: '>' symbol unexpected here");
                                }
                                tokens.push(BasicToken::FileRedirectOut { append: true });
                            } else {
                                tokens.push(BasicToken::FileRedirectOut { append: false });
                            }
                        } else if c == '<' {
                            tokens.push(BasicToken::FileRedirectIn);
                        }
                        continue;
                    }
                }
                QuoteType::Single => {
                    if c == '\'' {
                        tokens.push(BasicToken::Value(buffer.clone()));
                        buffer.clear();
                        quote_level = QuoteType::None;
                        continue;
                    }
                }
                QuoteType::Double => {
                    if c == '"' {
                        tokens.push(BasicToken::Value(buffer.clone()));
                        buffer.clear();
                        quote_level = QuoteType::None;
                        continue;
                    } else if c == '$' {
                        ignore_quotes = 1 + parse_variable(process, &mut source).await?;
                        continue;
                    }
                }
            }
        };
        buffer.push(c);
    }

    if !matches!(quote_level, QuoteType::None) {
        bail!("Mismatched quote");
    }
    if !buffer.is_empty() {
        tokens.push(BasicToken::Value(buffer.clone()));
    }

    // Assignment is kind of weird
    if let Some(BasicToken::Value(value)) = &tokens.get(0) {
        if value.contains('=') {
            let (identifier, value) = value.split_once('=').expect("Bug: expected equals sign");
            process.env.insert(identifier.into(), value.into());

            return Ok(tokens.into_iter().skip(1).collect());
        }
    }

    Ok(tokens)
}

fn dispatch(process: &mut Process, root: Token) -> BoxFuture<Result<()>> {
    async move {
        match root {
            Token::Command(args) => {
                if args.is_empty() {
                    bail!("Syntax error: Command cannot have empty arguments");
                }
                let command = args[0].clone();
                if command == "cd" {
                    shell_commands::cd(process, args).await?;
                } else if command == "env" {
                    shell_commands::env(process, args).await?;
                } else {
                    let mut process = process.clone();
                    process.args = args.clone();
                    if crate::programs::get_program(&mut process, args)
                        .await?
                        .is_none()
                    {
                        bail!("Command not found: {command}");
                    }
                }
                Ok(())
            }
            Token::Pipe(token1, token2) => {
                let (mut pin, pout, mut backend) = streams::pipe();

                let mut process1 = process.clone();
                process1.stdout = pout.clone();
                let mut process2 = process.clone();
                process2.stdin = pin.clone();

                try_join! {
                    backend.run(),
                    async {
                        dispatch(&mut process1, *token1).await?;
                        pout.shutdown().await?;
                        Ok(())
                    },
                    async {
                        dispatch(&mut process2, *token2).await?;
                        pin.shutdown().await?;
                        Ok(())
                    },
                }?;

                Ok(())
            }
            Token::FileRedirectOut { lhs, append, path } => {
                let (pout, mut backend) = {
                    let path = process.get_path(path)?;
                    let file = if append && path.exists()? {
                        path.append_file()?
                    } else {
                        path.create_file()?
                    };

                    streams::file_redirect_out(file)
                };

                let mut child_process = process.clone();
                child_process.stdout = pout.clone();

                try_join! {
                    backend.run(),
                    async {
                        dispatch(&mut child_process, *lhs).await?;
                        pout.shutdown().await?;
                        Ok(())
                    },
                }?;

                Ok(())
            }
            Token::FileRedirectIn { lhs, path } => {
                let (mut pin, mut backend) = {
                    let file = process.get_path(path)?.open_file()?;

                    streams::file_redirect_in(file)
                };

                let mut child_process = process.clone();
                child_process.stdin = pin.clone();

                try_join! {
                    backend.run(),
                    async {
                        dispatch(&mut child_process, *lhs).await?;
                        pin.shutdown().await?;
                        Ok(())
                    },
                }?;

                Ok(())
            }
        }
    }
    .boxed()
}

async fn await_abortable_future<T, F: Future<Output = Result<T>>>(
    mut abort_channel_rx: oneshot::Receiver<()>,
    future: F,
) -> Result<T> {
    let (meta_abort_channel_tx, mut meta_abort_channel_rx) = oneshot::channel::<()>();
    let (abort_handle, abort_registration) = AbortHandle::new_pair();
    let future = Abortable::new(future, abort_registration);
    let (_, result) = join! {
        async {
            select! {
                _ = abort_channel_rx => {
                    abort_handle.abort();
                },
                _ = meta_abort_channel_rx => {
                }
            };
        },
        async {
            let result = match future.await {
                Ok(inner) => inner,
                Err(_) =>  {
                    Err(anyhow!("INTERRUPT"))?
                }
            };
            let _ = meta_abort_channel_tx.send(());
            result
        }
    };
    result
}

pub fn run_script<'a>(process: &'a mut Process, source: &'a str) -> BoxFuture<'a, Result<()>> {
    async {
        let lines = source.split('\n');
        for line in lines {
            if line.trim().is_empty() {
                continue;
            }
            let tokens = tokenize(process, line).await?;
            if tokens.is_empty() {
                continue;
            }
            let root_token = parse(tokens)?;

            let (abort_channel_tx, abort_channel_rx) = oneshot::channel();
            process.signal_registrar.unbounded_send(abort_channel_tx)?;
            await_abortable_future(abort_channel_rx, dispatch(process, root_token)).await?;
        }
        Ok(())
    }
    .boxed()
}
/// Unix shell.
#[derive(Parser)]
struct Options {
    /// A command to run.
    #[arg(short, conflicts_with = "script")]
    command: Option<String>,
    /// A script to run.
    script: Option<String>,
}

pub async fn sh(process: &mut Process) -> Result<()> {
    let options = Options::try_parse_from(process.args.iter())?;

    let mut stdout = process.stdout.clone();
    let mut stdin = process.stdin.clone();

    if let Some(file_path) = options.script {
        let mut script = String::new();
        let mut process = process.clone();
        process
            .get_path(&file_path)?
            .open_file()?
            .read_to_string(&mut script)?;
        // Get rid of the 'sh' argument.
        process.args = process.args.iter().skip(1).cloned().collect();

        run_script(&mut process, &script).await?;
        return Ok(());
    }

    if let Some(command) = options.command {
        run_script(process, &command).await?;
        return Ok(());
    }

    let readline_history = FileBasedHistory::new(process.get_path(HISTORY_FILE)?);
    let mut readline = Readline::new(String::from("$ "), readline_history);

    let bin_paths: Result<Vec<VfsPath>> = process
        .env
        .get("PATH")
        .ok_or_else(|| anyhow!("Could not get PATH variable"))?
        .split(':')
        .map(|path| process.get_path(path))
        .collect();
    let bin_paths = bin_paths?;

    loop {
        let tab_completer = |section: String, start: usize| {
            let word = &section[start..];
            let words: Vec<&str> = section.split_whitespace().collect();
            let mut suggestions = Vec::new();

            // Commands occur at start of line, or after pipes
            if (words.is_empty() || !section.ends_with(' '))
                && (words.len() < 2 || words[words.len() - 2] == "|")
            {
                for path in bin_paths.clone() {
                    for command in path.read_dir()? {
                        let mut filename = command.filename();
                        if command.is_file()? && filename.starts_with(word) {
                            filename.push(' ');
                            suggestions.push(filename);
                        }
                    }
                }
            } else {
                let (path, file) = if let Some(slash) = word.rfind('/') {
                    (&word[0..slash], &word[slash + 1..])
                } else {
                    ("", word)
                };
                let path = process.get_path(path)?;

                for dir in path.read_dir()? {
                    if dir.filename().starts_with(file) {
                        let mut suggestion = dir.as_str().to_string();
                        if dir.is_file()? {
                            suggestion.push(' ');
                        } else {
                            suggestion.push('/');
                        }
                        suggestions.push(suggestion);
                    }
                }
            }

            Ok(suggestions)
        };

        let (abort_channel_tx, abort_channel_rx) = oneshot::channel();
        process.signal_registrar.unbounded_send(abort_channel_tx)?;
        let line: String = match await_abortable_future::<String, _>(
            abort_channel_rx,
            readline.get_line(&mut stdin, &mut stdout, tab_completer),
        )
        .await
        {
            Ok(line) => line,
            Err(e) => {
                process.stderr.write_all(b"\nreadline: ").await?;
                process.stderr.write_all(e.to_string().as_bytes()).await?;
                process.stderr.write_all(b"\n").await?;
                continue;
            }
        };
        if line.trim().is_empty() {
            continue;
        }

        if let Err(e) = run_script(process, &line).await {
            process.stderr.write_all(e.to_string().as_bytes()).await?;
            process.stderr.write_all(b"\n").await?;
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use futures::channel::mpsc;
    use vfs::MemoryFS;

    fn make_process() -> Process {
        let (stdin, stdout, _) = streams::pipe();
        let (signal_registrar, _) = mpsc::unbounded();
        let stderr = stdout.clone();
        let cwd: VfsPath = MemoryFS::new().into();
        Process {
            stdin,
            stderr,
            stdout,
            signal_registrar,
            cwd,
            args: Vec::new(),
            env: Default::default(),
        }
    }

    #[futures_test::test]
    async fn variables() {
        let mut process = make_process();
        process.env.insert("foo".into(), "FOO".into());
        process.env.insert("bar".into(), "BAR".into());
        process.env.insert("baz".into(), "BAZ".into());
        let source = "echo ${foo} ${bar}${baz}";
        let tokens = tokenize(&mut process, source).await.unwrap();
        let expected = vec![
            BasicToken::Value("echo".into()),
            BasicToken::Value("FOO".into()),
            BasicToken::Value("BARBAZ".into()),
        ];
        assert_eq!(tokens, expected);
    }

    #[futures_test::test]
    async fn tokenize_pipe() {
        let mut process = make_process();
        let source = "echo\thi '|'   there | cowsay";
        let tokens = tokenize(&mut process, source).await.unwrap();
        let expected = vec![
            BasicToken::Value("echo".into()),
            BasicToken::Value("hi".into()),
            BasicToken::Value("|".into()),
            BasicToken::Value("there".into()),
            BasicToken::Pipe,
            BasicToken::Value("cowsay".into()),
        ];
        assert_eq!(tokens, expected);
    }

    #[futures_test::test]
    async fn tokenize_fileio() {
        let mut process = make_process();
        let source = "fortune >> waa";
        let tokens = tokenize(&mut process, source).await.unwrap();
        let expected = vec![
            BasicToken::Value("fortune".into()),
            BasicToken::FileRedirectOut { append: true },
            BasicToken::Value("waa".into()),
        ];
        assert_eq!(tokens, expected);

        let source = "fortune > waa";
        let tokens = tokenize(&mut process, source).await.unwrap();
        let expected = vec![
            BasicToken::Value("fortune".into()),
            BasicToken::FileRedirectOut { append: false },
            BasicToken::Value("waa".into()),
        ];
        assert_eq!(tokens, expected);
    }
}
