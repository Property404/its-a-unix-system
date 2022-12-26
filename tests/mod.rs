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
    tester.run("export foo=bar")?;
    tester.run("echo ${foo}${foo}")?;
    tester.expect("barbar")?;
    tester.run("export foo=foo${foo}")?;
    tester.run("echo ${foo}")?;
    tester.expect("foobar")?;
    // Make sure quoting works as expected
    tester.run("export foo=\"y'all\tare ugly\"")?;
    tester.run("echo \"${foo}\"")?;
    tester.expect("y'all\tare ugly")?;
    tester.run("echo ${foo}")?;
    tester.expect("y'all are ugly")?;
    // ...and with double quotes inside
    tester.run("export foo='quote \"'")?;
    tester.run("echo \"${foo}\"")?;
    tester.expect("quote \"")?;
    // And that we don't recurse shell vars
    tester.run("sh -c 'echo -- ${2}'")?;
    tester.expect("echo -- ${2}")?;

    // && and ||
    tester.run("false || echo false")?;
    tester.expect("false")?;
    tester.run("true || echo butts")?;
    tester.run("true && echo true")?;
    tester.expect("true")?;
    // Tests
    tester.run("test a == a && echo yes")?;
    tester.expect("yes")?;
    tester.run("test a != a || echo no")?;
    tester.expect("no")?;
    tester.run("test ! a != a || echo alpha")?;
    tester.run("test ! a != a && echo beta")?;
    tester.expect("beta")?;
    tester.run("test yes =~ y && echo yes")?;
    tester.expect("yes")?;

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
                        programs::shell(&mut shell).await.unwrap();
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
