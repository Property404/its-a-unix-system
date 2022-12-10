//! Integration tests.
use anyhow::Result;
use faux_unix_system::{filesystem, process::Process, programs, streams};
use futures::{
    channel::mpsc::{self, UnboundedSender},
    stream::StreamExt,
    try_join,
};

#[derive(PartialEq, Eq)]
enum Command {
    Run(String),
    Expect(String),
}

struct Tester(UnboundedSender<Command>);
impl Tester {
    fn run(&self, value: &str) -> Result<()> {
        self.0.unbounded_send(Command::Run(value.into()))?;
        Ok(())
    }

    fn expect(&self, value: &str) -> Result<()> {
        self.0.unbounded_send(Command::Expect(value.into()))?;
        Ok(())
    }
}

async fn integration_test_inner(tx: UnboundedSender<Command>) -> Result<()> {
    let tester = Tester(tx);

    // Basic
    tester.run("echo hi | tee a")?;
    tester.expect("hi")?;
    tester.run("cat a")?;
    tester.expect("hi")?;
    tester.run("cat < a")?;
    tester.expect("hi")?;
    tester.run("echo hello >> a")?;
    tester.run("cat a | sort")?;
    tester.expect("hello")?;
    tester.expect("hi")?;
    tester.run("rm a")?;

    // Environmental variables
    tester.run("foo=bar")?;
    tester.run("echo ${foo}${foo}")?;
    tester.expect("barbar")?;
    tester.run("foo=foo${foo}")?;
    tester.run("echo ${foo}")?;
    tester.expect("foobar")?;

    Ok(())
}

#[futures_test::test]
async fn integration_test() -> Result<()> {
    let (mut stdin, stdin_tx, mut stdin_backend) = streams::pipe();
    let (mut stdout_rx, stdout, mut stdout_backend) = streams::pipe();
    let (signal_registrar, mut signal_registrar_tx) = mpsc::unbounded();
    let (command_tx, mut command_rx) = mpsc::unbounded();
    let rootfs = filesystem::get_root()?;

    let mut shell = Process {
        stdin: stdin.clone(),
        stdout: stdout.clone(),
        stderr: stdout.clone(),
        env: Default::default(),
        signal_registrar,
        cwd: rootfs,
        args: vec!["-sh".into()],
    };
    shell.env.insert("PATH".into(), "bin".into());

    try_join!(
        stdin_backend.run(),
        stdout_backend.run(),
        integration_test_inner(command_tx),
        async {
            while let Some(command) = command_rx.next().await {
                match command {
                    Command::Run(value) => {
                        shell.args = vec!["sh".into(), "-c".into(), value];
                        programs::shell(&mut shell).await.unwrap()
                    }
                    Command::Expect(value) => {
                        assert_eq!(value, stdout_rx.get_line().await?);
                    }
                }
            }

            // Could be concurrent
            stdout.shutdown().await?;
            stdin.shutdown().await?;
            stdout_rx.shutdown().await?;
            stdin_tx.shutdown().await?;
            signal_registrar_tx.close();
            Ok(())
        }
    )?;
    Ok(())
}
