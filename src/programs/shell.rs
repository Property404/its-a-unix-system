use crate::{process::Process, streams};
use anyhow::{anyhow, bail, Error, Result};
use futures::{
    channel::oneshot,
    future::{BoxFuture, FutureExt},
    io::AsyncWriteExt,
    select,
    stream::{AbortHandle, Abortable},
    try_join,
};

#[derive(Debug, PartialEq, Eq)]
enum BasicToken {
    Pipe,
    Value(String),
}

enum QuoteType {
    None,
    Single,
    Double,
}

enum Token {
    Pipe(Box<Token>, Box<Token>),
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
                    Token::Pipe(_, _) => unreachable!("Pipes cannot be nested on rhs"),
                },
                Token::Command(values) => values.push(value),
            },
            BasicToken::Pipe => {
                root = Token::Pipe(Box::new(root), Box::new(Token::Command(Vec::new())));
            }
        }
    }

    Ok(root)
}

fn tokenize(source: &str) -> Result<Vec<BasicToken>> {
    let mut quote_level = QuoteType::None;
    let mut tokens = Vec::new();
    let mut buffer = String::new();
    for c in source.chars() {
        match quote_level {
            QuoteType::None => {
                if c == '\'' {
                    quote_level = QuoteType::Single;
                    continue;
                } else if c == '"' {
                    quote_level = QuoteType::Double;
                    continue;
                } else if [' ', '\n', '\t', '|'].contains(&c) {
                    if !buffer.is_empty() {
                        tokens.push(BasicToken::Value(buffer.clone()));
                        buffer.clear();
                    }
                    if c == '|' {
                        tokens.push(BasicToken::Pipe);
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
                let command = args[0].clone();
                if command == "cd" {
                    if args.len() > 1 {
                        let new_path = process.cwd.join(&args[1])?;
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
                    .await
                    .transpose()?
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
        let root_token = parse(tokenize(line)?)?;

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

pub async fn shell(process: &mut Process, args: Vec<String>) -> Result<()> {
    let mut stdout = process.stdout.clone();
    let mut stdin = process.stdin.clone();

    if args.len() > 1 {
        let mut script = String::new();
        process
            .cwd
            .join(&args[1])?
            .open_file()?
            .read_to_string(&mut script)?;

        run_script(process, &script).await?;
        return Ok(());
    }

    loop {
        stdout.write_all(b"$ ").await?;
        stdout.flush().await?;

        let line = stdin.get_line().await?;
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
    fn tokenize_source() {
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
}
