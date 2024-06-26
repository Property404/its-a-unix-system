use crate::{
    filesystem,
    process::{ExitCode, Process},
    programs::common::{
        extendable_iterator::ExtendableIterator,
        readline::{FileBasedHistory, Readline},
        shell_commands,
    },
    streams,
};
use anyhow::{anyhow, bail, Result};
use ascii::AsciiChar;
use clap::Parser;
use futures::{
    channel::oneshot,
    future::{BoxFuture, FutureExt},
    io::{AsyncReadExt, AsyncWriteExt},
    join, select,
    stream::{AbortHandle, Abortable},
    try_join,
};
use std::{collections::HashMap, future::Future};
use vfs::VfsPath;

const HISTORY_FILE: &str = "/etc/.sh_history";

#[derive(Default, Clone)]
pub struct ShellContext {
    pub variables: HashMap<String, String>,
    pub do_exit_with: Option<ExitCode>,
}

enum AbortableResult<T> {
    Completed(Result<T>),
    Aborted,
}

impl<T> AbortableResult<T> {
    fn fail_if_aborted(self) -> Result<T> {
        match self {
            AbortableResult::Completed(result) => result,
            AbortableResult::Aborted => bail!("INTERRUPT"),
        }
    }

    fn completed_or(self, value: Result<T>) -> Result<T> {
        match self {
            AbortableResult::Completed(result) => result,
            AbortableResult::Aborted => value,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum BasicToken {
    And,
    Or,
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
    And(Box<Token>, Box<Token>),
    Or(Box<Token>, Box<Token>),
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

    let mut basic_tokens = basic_tokens.into_iter();

    while let Some(basic) = basic_tokens.next() {
        match basic {
            BasicToken::Value(value) => match &mut root {
                Token::And(_, _) | Token::Or(_, _) => {
                    unreachable!("Bug: &&/|| should not be accessible at this point");
                }
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
            BasicToken::And => {
                let rest = basic_tokens.collect();
                return Ok(Token::And(Box::new(root), Box::new(parse(rest)?)));
            }
            BasicToken::Or => {
                let rest = basic_tokens.collect();
                return Ok(Token::Or(Box::new(root), Box::new(parse(rest)?)));
            }
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
async fn parse_variable(
    ctx: &mut ShellContext,
    process: &Process,
    source: &mut ExtendableIterator<char>,
) -> Result<usize> {
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
                        run_script(ctx, &mut process, &value).await?;
                        stdout.flush().await?;
                        stdout.shutdown().await?;
                        reader.read_to_string(&mut output).await?;
                        reader.shutdown().await?;
                        Ok(output)
                    },
                };
                output
            };
            let output = await_abortable_future(abort_channel_rx, future)
                .await
                .fail_if_aborted()?;

            source.prepend(output.chars());
            return Ok(output.len());
        // Variables
        } else if delimiter == '{' && c == '}' {
            let value = if let Some(value) = process.env.get(&value) {
                value.to_string()
            } else if let Some(value) = ctx.variables.get(&value) {
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

async fn tokenize(
    ctx: &mut ShellContext,
    process: &mut Process,
    source: &mut ExtendableIterator<char>,
) -> Result<Vec<BasicToken>> {
    let mut quote_level = QuoteType::None;
    let mut tokens = Vec::new();
    let mut buffer = String::new();
    // We don't consider quote changes or start of variables when we're inside the result of a
    // variable/subshell
    let mut ignore_quotes: usize = 0;
    // Current character
    let mut c = None;

    loop {
        let last_char = c;
        c = source.next();
        let Some(c) = c else {
            break;
        };
        ignore_quotes = ignore_quotes.saturating_sub(1);

        if quote_level == QuoteType::None && [' ', '\n', '\t'].contains(&c) {
            if !buffer.is_empty() {
                tokens.push(BasicToken::Value(buffer.clone()));
                buffer.clear();
            }
            if c == '\n' && ignore_quotes == 0 {
                break;
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
                        loop {
                            let next = source.next();
                            if next.is_none() || next == Some('\n') {
                                break;
                            }
                        }
                        break;
                    } else if c == '\\' {
                        if let Some(next) = source.next() {
                            if next != '\n' {
                                buffer.push(next);
                            }
                        } else {
                            break;
                        }
                        continue;
                    } else if ['&', '|', '>', '<', ';'].contains(&c) {
                        if !buffer.is_empty() {
                            tokens.push(BasicToken::Value(buffer.clone()));
                            buffer.clear();
                        }
                        if c == ';' {
                            break;
                        } else if c == '|' {
                            if last_char == Some('|') {
                                if !matches!(tokens.pop(), Some(BasicToken::Pipe)) {
                                    bail!("Syntax error: Unexpected pipe");
                                }
                                tokens.push(BasicToken::Or);
                            } else {
                                tokens.push(BasicToken::Pipe);
                            }
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
                        } else if c == '&' {
                            if source.next() != Some('&') {
                                bail!("Syntax error: Background tasks not supported");
                            }
                            tokens.push(BasicToken::And);
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
            }

            if quote_level == QuoteType::None || quote_level == QuoteType::Double {
                // Subsitute home directory
                if c == '~'
                    && (last_char
                        .map(|c| c.is_whitespace() || c == '"')
                        .unwrap_or(true))
                {
                    let next_c = source.next();
                    if let Some(next_c) = next_c {
                        source.prepend(vec![next_c].into_iter());
                    }
                    if next_c
                        .map(|c| c.is_whitespace() || c == '/' || c == '"')
                        .unwrap_or(true)
                    {
                        source.prepend(
                            process
                                .env
                                .get("HOME")
                                .cloned()
                                .unwrap_or_else(|| "~".into())
                                .chars(),
                        )
                    }
                    continue;
                } else if c == '$' {
                    // Extra 1 because we sub on beginning of loop
                    ignore_quotes = 1 + parse_variable(ctx, process, source).await?;
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

    // Assignment is kind of weird
    if let Some(BasicToken::Value(value)) = &tokens.first() {
        if value.contains('=') {
            let (identifier, value) = value.split_once('=').expect("Bug: expected equals sign");

            ctx.variables.insert(identifier.into(), value.into());
            if process.env.contains_key(identifier) {
                process.env.insert(identifier.into(), value.into());
            }

            return Ok(tokens.into_iter().skip(1).collect());
        }
    }

    Ok(tokens)
}

fn dispatch<'a>(
    ctx: &'a mut ShellContext,
    process: &'a mut Process,
    root: Token,
) -> BoxFuture<'a, Result<ExitCode>> {
    async move {
        match root {
            Token::Command(args) => {
                if args.is_empty() {
                    bail!("Syntax error: Command cannot have empty arguments");
                }
                let command = args[0].clone();
                if command == "cd" {
                    shell_commands::cd(process, args).await
                } else if command == "env" {
                    shell_commands::env(process, args).await
                } else if command == "exec" {
                    shell_commands::exec(ctx, process, args).await
                } else if command == "exit" {
                    shell_commands::exit(ctx, process, args).await
                } else if command == "export" {
                    shell_commands::export(ctx, process, args).await
                } else if command == "read" {
                    shell_commands::read(ctx, process, args).await
                } else if command == "source" || command == "." {
                    shell_commands::source(ctx, process, args).await
                } else if command == "true" {
                    Ok(ExitCode::SUCCESS)
                } else if command == "false" {
                    Ok(ExitCode::FAILURE)
                } else {
                    let mut process = process.clone();
                    process.args.clone_from(&args);
                    match crate::programs::exec_program(&mut process, &command).await? {
                        None => {
                            process
                                .stderr
                                .write_all(format!("Command not found: {command}\n").as_bytes())
                                .await?;
                            Ok(ExitCode::FAILURE)
                        }
                        Some(code) => Ok(code),
                    }
                }
            }
            Token::Pipe(token1, token2) => {
                let (mut pin, pout, mut backend) = streams::pipe();

                let mut process1 = process.clone();
                process1.stdout = pout.clone();
                let mut process2 = process.clone();
                process2.stdin = pin.clone();

                // Prevent Broken Pipe errors.
                let (abort_channel_tx, abort_channel_rx) = oneshot::channel();
                let (meta_abort_channel_tx, meta_abort_channel_rx) = oneshot::channel::<()>();

                let (_, _, result) = try_join! {
                    backend.run(),
                    async {
                        let result = await_abortable_future(
                            abort_channel_rx,
                            dispatch(&mut Default::default(), &mut process1, *token1))
                            .await.completed_or(Ok(ExitCode::FAILURE));
                        let _ = meta_abort_channel_tx.send(());
                        pout.shutdown().await?;
                        result
                    },
                    async {
                        let result = dispatch(&mut Default::default(), &mut process2, *token2).await;
                        let _ = abort_channel_tx.send(());
                        // give a chance for the sibling to be aborted before
                        // shutting down the input stream
                        meta_abort_channel_rx.await?;
                        pin.shutdown().await?;
                        result
                    },
                }?;

                Ok(result)
            }
            Token::And(token1, token2) => {
                let result = dispatch(ctx, process, *token1).await?;
                if result.is_success() {
                    dispatch(ctx, process, *token2).await
                } else {
                    Ok(result)
                }
            }
            Token::Or(token1, token2) => {
                let result = dispatch(ctx, process, *token1).await?;
                if result.is_failure() {
                    dispatch(ctx, process, *token2).await
                } else {
                    Ok(result)
                }
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

                let (_, result) = try_join! {
                    backend.run(),
                    async {
                        let result = dispatch(ctx, &mut child_process, *lhs).await;
                        pout.shutdown().await?;
                        result
                    },
                }?;

                Ok(result)
            }
            Token::FileRedirectIn { lhs, path } => {
                let (mut pin, mut backend) = {
                    let file = process.get_path(path)?.open_file()?;

                    streams::file_redirect_in(file)
                };

                let mut child_process = process.clone();
                child_process.stdin = pin.clone();

                let (_, result) = try_join! {
                    backend.run(),
                    async {
                        let result = dispatch(ctx, &mut child_process, *lhs).await;
                        pin.shutdown().await?;
                        result
                    },
                }?;

                Ok(result)
            }
        }
    }
    .boxed()
}

async fn await_abortable_future<T, F: Future<Output = Result<T>>>(
    mut abort_channel_rx: oneshot::Receiver<()>,
    future: F,
) -> AbortableResult<T> {
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
                Ok(inner) => AbortableResult::Completed(inner),
                Err(_) =>  {
                    AbortableResult::Aborted
                }
            };
            let _ = meta_abort_channel_tx.send(());
            result
        }
    };
    result
}

pub fn run_script<'a>(
    ctx: &'a mut ShellContext,
    process: &'a mut Process,
    source: &'a str,
) -> BoxFuture<'a, Result<ExitCode>> {
    async {
        let mut result = ExitCode::SUCCESS;
        let mut it = ExtendableIterator::new(source.chars());

        while !it.is_empty() {
            let tokens = tokenize(ctx, process, &mut it).await?;
            if tokens.is_empty() {
                continue;
            }
            let root_token = parse(tokens)?;

            let (abort_channel_tx, abort_channel_rx) = oneshot::channel();
            process.signal_registrar.unbounded_send(abort_channel_tx)?;

            result = await_abortable_future(abort_channel_rx, dispatch(ctx, process, root_token))
                .await
                .fail_if_aborted()?;
            if let Some(exit_code) = ctx.do_exit_with {
                return Ok(exit_code);
            }
        }
        Ok(result)
    }
    .boxed()
}

/// Unix shell.
#[derive(Parser)]
struct Options {
    /// A command to run.
    #[arg(short, conflicts_with = "script")]
    command: Option<String>,
    /// A script to source.
    #[arg(short, conflicts_with = "script", conflicts_with = "command")]
    source: Option<String>,
    /// A script to run.
    script: Option<String>,
}

pub async fn sh(process: &mut Process) -> Result<ExitCode> {
    let options = Options::try_parse_from(process.args.iter())?;

    let mut stdout = process.stdout.clone();
    let mut stdin = process.stdin.clone();

    let mut ctx = ShellContext::default();

    if let Some(file_path) = options.script {
        let mut script = String::new();
        let mut process = process.clone();
        process
            .get_path(&file_path)?
            .open_file()?
            .read_to_string(&mut script)?;
        // Get rid of the 'sh' argument.
        process.args = process.args.iter().skip(1).cloned().collect();

        run_script(&mut ctx, &mut process, &script).await?;
        return Ok(ExitCode::SUCCESS);
    }

    if let Some(command) = options.command {
        run_script(&mut ctx, process, &command).await?;
        return Ok(ExitCode::SUCCESS);
    }

    if let Some(file_path) = options.source {
        let mut script = String::new();
        process
            .get_path(&file_path)?
            .open_file()?
            .read_to_string(&mut script)?;
        run_script(&mut ctx, process, &script).await?;
    }

    let readline_history = FileBasedHistory::new(process.get_path(HISTORY_FILE)?);
    let mut readline = Readline::new(readline_history);

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
            let mut skip_path_completion = false;

            // Commands occur at start of line, or after pipes
            if (words.is_empty() || !section.ends_with(' '))
                && (words.len() < 2 || ["|", "&&", "||", ";"].contains(&words[words.len() - 2]))
            {
                skip_path_completion = true;
                // External(as in, not part of the shell) commands.
                for path in bin_paths.clone() {
                    for command in path.read_dir()? {
                        let mut filename = command.filename();
                        if command.is_file()? && filename.starts_with(word) {
                            filename.push(' ');
                            suggestions.push(filename);
                        }
                    }
                }

                // Shell commands
                for command in shell_commands::COMMANDS {
                    if command.starts_with(&section) {
                        let mut command = String::from(command);
                        command.push(' ');
                        suggestions.push(command);
                    }
                }
            }

            if word.starts_with('/')
                || word.starts_with("./")
                || word.starts_with("../")
                || word.starts_with("~/")
            {
                skip_path_completion = false;
            }

            // Path completion
            if !skip_path_completion {
                let (path_str, file) = if let Some(slash) = word.rfind('/') {
                    (
                        if slash == 0 { "/" } else { &word[0..slash] },
                        &word[slash + 1..],
                    )
                } else {
                    ("", word)
                };

                let Some(home) = process.env.get("HOME") else {
                    bail!("No ${{HOME}} environmental variable");
                };
                let path = process.get_path(path_str.replace('~', home))?;

                if path.exists()? {
                    for entity in path.read_dir()? {
                        let filename = entity.filename();
                        if filename.starts_with(file) {
                            let mut suggestion = if path_str.is_empty() {
                                filename
                            } else if path_str == "/" {
                                format!("/{filename}")
                            } else {
                                format!("{path_str}/{filename}")
                            };

                            if entity.is_file()? {
                                suggestion.push(' ');
                            } else {
                                suggestion.push('/');
                            }
                            suggestions.push(suggestion);
                        }
                    }
                }
            }

            Ok(suggestions)
        };

        // User-specified prompt
        let prompt = if let Some(ps1) = process.env.get("PS1") {
            let mut prompt = String::new();
            let mut backslash = false;
            for c in ps1.chars() {
                if backslash {
                    match c {
                        // PWD
                        'w' => {
                            let cwd = filesystem::vfs_path_to_str(&process.cwd);
                            prompt.push_str(cwd);
                        }
                        // Folder name
                        'W' => {
                            let cwd = if process.cwd.is_root() {
                                "/".into()
                            } else {
                                process.cwd.filename()
                            };
                            prompt.push_str(&cwd);
                        }
                        // Backslash
                        '\\' => {
                            prompt.push('\\');
                        }
                        // Newline
                        'n' => {
                            prompt.push('\n');
                        }
                        // Carriage return
                        'r' => {
                            prompt.push('\r');
                        }
                        // Escape
                        'e' => {
                            prompt.push(AsciiChar::ESC.as_char());
                        }
                        _ => {}
                    }
                    backslash = false;
                } else if c == '\\' {
                    backslash = true;
                } else {
                    prompt.push(c);
                }
            }

            prompt
        } else {
            String::from("$ ")
        };

        let (abort_channel_tx, abort_channel_rx) = oneshot::channel();
        process.signal_registrar.unbounded_send(abort_channel_tx)?;
        let line: String = match await_abortable_future::<String, _>(
            abort_channel_rx,
            readline.get_line(&prompt, &mut stdin, &mut stdout, tab_completer),
        )
        .await
        {
            AbortableResult::Completed(Ok(line)) => line,
            AbortableResult::Completed(Err(e)) => {
                process.stderr.write_all(b"\nreadline: ").await?;
                process.stderr.write_all(e.to_string().as_bytes()).await?;
                process.stderr.write_all(b"\n").await?;
                continue;
            }
            AbortableResult::Aborted => {
                process.stdout.write_all(b"\n").await?;
                continue;
            }
        };
        if line.trim().is_empty() {
            continue;
        }

        if let Err(e) = run_script(&mut ctx, process, &line).await {
            process.stderr.write_all(e.to_string().as_bytes()).await?;
            process.stderr.write_all(b"\n").await?;
        }

        if let Some(exit_code) = ctx.do_exit_with {
            return Ok(exit_code);
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
        let mut ctx = Default::default();
        process.env.insert("foo".into(), "FOO".into());
        process.env.insert("bar".into(), "BAR".into());
        process.env.insert("baz".into(), "BAZ".into());
        let source = "echo ${foo} ${bar}${baz}";
        let tokens = tokenize(
            &mut ctx,
            &mut process,
            &mut ExtendableIterator::new(source.chars()),
        )
        .await
        .unwrap();
        let expected = vec![
            BasicToken::Value("echo".into()),
            BasicToken::Value("FOO".into()),
            BasicToken::Value("BARBAZ".into()),
        ];
        assert_eq!(tokens, expected);
    }

    #[futures_test::test]
    async fn tokenize_pipe() {
        let mut ctx = Default::default();
        let mut process = make_process();
        let source = "echo\thi '|'   there | cowsay";
        let tokens = tokenize(
            &mut ctx,
            &mut process,
            &mut ExtendableIterator::new(source.chars()),
        )
        .await
        .unwrap();
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
        let mut ctx = Default::default();
        let source = "fortune >> waa";
        let tokens = tokenize(
            &mut ctx,
            &mut process,
            &mut ExtendableIterator::new(source.chars()),
        )
        .await
        .unwrap();
        let expected = vec![
            BasicToken::Value("fortune".into()),
            BasicToken::FileRedirectOut { append: true },
            BasicToken::Value("waa".into()),
        ];
        assert_eq!(tokens, expected);

        let source = "fortune > waa";
        let tokens = tokenize(
            &mut ctx,
            &mut process,
            &mut ExtendableIterator::new(source.chars()),
        )
        .await
        .unwrap();
        let expected = vec![
            BasicToken::Value("fortune".into()),
            BasicToken::FileRedirectOut { append: false },
            BasicToken::Value("waa".into()),
        ];
        assert_eq!(tokens, expected);
    }
}
