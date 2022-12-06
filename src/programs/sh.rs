use crate::{
    process::Process,
    programs::common::readline::{FileBasedHistory, Readline},
    streams,
};
use vfs::VfsPath;
use anyhow::{anyhow, bail, Error, Result};
use futures::{
    channel::oneshot,
    future::{BoxFuture, FutureExt},
    io::AsyncWriteExt,
    select,
    stream::{AbortHandle, Abortable},
    try_join,
};

const HISTORY_FILE: &str = "/etc/.sh_history";

#[derive(Debug, PartialEq, Eq)]
enum BasicToken {
    Pipe,
    FileRedirectOut { append: bool },
    FileRedirectIn,
    Value(String),
}

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

fn tokenize(source: &str) -> Result<Vec<BasicToken>> {
    let mut quote_level = QuoteType::None;
    let mut tokens = Vec::new();
    let mut buffer = String::new();
    let mut source = source.chars();
    let mut c = None;
    loop {
        let last_char = c;
        c = source.next();
        let Some(c) = c else {break;};

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
                } else if [' ', '\n', '\t', '|', '>', '<'].contains(&c) {
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
                    if args.len() > 1 {
                        let new_path = process.get_path(&args[1])?;
                        if new_path.is_dir()? {
                            process.cwd = new_path;
                        } else {
                            process.stderr.write_all(b"cd: ").await?;
                            process
                                .stdout
                                .write_all(new_path.as_str().as_bytes())
                                .await?;
                            process.stderr.write_all(b": No such directory\n").await?;
                        }
                    }
                } else if crate::programs::get_program(process, args)
                    .await?
                    .is_none()
                {
                    bail!("Command not found: {command}");
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

async fn run_script(process: &mut Process, source: &str) -> Result<()> {
    let lines = source.split('\n');
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let tokens = tokenize(line)?;
        if tokens.is_empty() {
            continue;
        }
        let root_token = parse(tokens)?;

        let (abort_channel_tx, mut abort_channel_rx) = oneshot::channel();
        let (meta_abort_channel_tx, mut meta_abort_channel_rx) = oneshot::channel();

        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        process.signal_registrar.unbounded_send(abort_channel_tx)?;
        let future = Abortable::new(dispatch(process, root_token), abort_registration);
        try_join! {
            async {
                select! {
                    _ = abort_channel_rx => {
                        abort_handle.abort();
                    },
                    _ = meta_abort_channel_rx => {
                    }
                };
                Result::<(), Error>::Ok(())
            },
            async {
                match future.await {
                    Ok(inner) => inner?,
                    Err(_) =>  {
                        Err(anyhow!("INTERRUPT"))?
                    }
                };
                let _ = meta_abort_channel_tx.send(());
                Ok(())
            }
        }?;
    }
    Ok(())
}

pub async fn sh(process: &mut Process, args: Vec<String>) -> Result<()> {
    let mut stdout = process.stdout.clone();
    let mut stdin = process.stdin.clone();

    if args.len() > 1 {
        let mut script = String::new();
        process
            .get_path(&args[1])?
            .open_file()?
            .read_to_string(&mut script)?;

        run_script(process, &script).await?;
        return Ok(());
    }

    let readline_history = FileBasedHistory::new(process.get_path(HISTORY_FILE)?);
    let mut readline = Readline::new(String::from("$ "), readline_history);

    let bin_paths: Result<Vec<VfsPath>> = process.env.get("PATH")
        .ok_or_else(||anyhow!("Could not get PATH variable"))?
        .split(':')
        .map(|path| process.get_path(path))
        .collect();
    let bin_paths = bin_paths?;

    loop {
        let line = match readline.get_line(&mut stdin, &mut stdout, Some(|section: String, start: usize|{
            let word = &section[start..];
            let words: Vec<&str> = section.split_whitespace().collect();
            let mut suggestions = Vec::new();

            // Commands occur at start of line, or after pipes
            if words.len() < 2 || words[words.len() - 2] == "|" {
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
                    (&word[0..slash],&word[slash+1..])
                } else {
                    ("", word)
                };
                let path = process.get_path(path)?;

                for dir in path.read_dir()? {
                    if dir.filename().starts_with(file) {
                        suggestions.push(dir.as_str().to_string());
                    }
                }
            }

            Ok(suggestions)
        })).await {
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

    #[test]
    fn tokenize_pipe() {
        let source = "echo\thi '|'   there | cowsay";
        let tokens = tokenize(source).unwrap();
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

    #[test]
    fn tokenize_fileio() {
        let source = "fortune >> waa";
        let tokens = tokenize(source).unwrap();
        let expected = vec![
            BasicToken::Value("fortune".into()),
            BasicToken::FileRedirectOut { append: true },
            BasicToken::Value("waa".into()),
        ];
        assert_eq!(tokens, expected);

        let source = "fortune > waa";
        let tokens = tokenize(source).unwrap();
        let expected = vec![
            BasicToken::Value("fortune".into()),
            BasicToken::FileRedirectOut { append: false },
            BasicToken::Value("waa".into()),
        ];
        assert_eq!(tokens, expected);
    }
}
