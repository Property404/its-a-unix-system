mod ansi_codes;
pub mod filesystem;
mod generated;
pub mod process;
pub mod programs;
pub mod streams;
mod utils;
use ansi_codes::{AnsiCode, ControlChar};
use anyhow::Result;
use futures::{io::AsyncWriteExt, try_join};
use process::Process;
use wasm_bindgen::prelude::*;

const PROFILE_PATH: &str = "/etc/profile";
const HOME_PATH: &str = "/root";
const BIN_PATHS: &str = "/bin:/usr/bin";
const USER: &str = "root";

async fn run() -> Result<()> {
    utils::set_panic_hook();

    let (stdin, stdout, mut backend, signal_registrar) = streams::standard()?;

    let rootfs = filesystem::get_root()?;
    let mut process = Process {
        stdin: stdin.clone(),
        stdout: stdout.clone(),
        stderr: stdout.clone(),
        env: Default::default(),
        signal_registrar,
        cwd: rootfs.join(HOME_PATH)?,
        args: vec!["-sh".into(), "-s".into(), PROFILE_PATH.into()],
    };

    for (key, value) in [("USER", USER), ("HOME", HOME_PATH), ("PATH", BIN_PATHS)] {
        process.env.insert(key.into(), value.into());
    }

    try_join!(backend.run(), async {
        loop {
            let mut child = process.clone();
            programs::exec_program(&mut child, "sh").await?;
            process
                .stderr
                .write_all(b"Oops! Looks like you exited your shell.\n")
                .await?;
            process
                .stderr
                .write_all(b"Let me get a fresh one for you.\n")
                .await?;
        }
        // So return type can be inferred.
        #[allow(unreachable_code)]
        Ok(())
    })?;
    Ok(())
}

#[wasm_bindgen]
pub async fn begin() {
    run().await.unwrap()
}
