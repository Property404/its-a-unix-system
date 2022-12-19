mod ansi_codes;
pub mod filesystem;
mod generated;
pub mod process;
pub mod programs;
pub mod streams;
mod utils;
use ansi_codes::{AnsiCode, ControlChar};
use anyhow::Result;
use futures::try_join;
use process::Process;
use wasm_bindgen::prelude::*;

const DEFAULT_SEARCH_PATHS: [&str; 2] = ["bin", "usr/bin"];

async fn run() -> Result<()> {
    utils::set_panic_hook();

    let (mut stdin, stdout, mut backend, signal_registrar) = streams::standard()?;

    let rootfs = filesystem::get_root()?;
    let mut shell = Process {
        stdin: stdin.clone(),
        stdout: stdout.clone(),
        stderr: stdout.clone(),
        env: Default::default(),
        signal_registrar,
        cwd: rootfs,
        args: vec!["-sh".into()],
        do_exit_with: None,
    };
    shell
        .env
        .insert("PATH".into(), DEFAULT_SEARCH_PATHS.join(":"));

    try_join!(backend.run(), async {
        programs::shell(&mut shell).await?;
        // Could be concurrent
        stdout.shutdown().await?;
        stdin.shutdown().await?;
        Ok(())
    })?;
    Ok(())
}

#[wasm_bindgen]
pub async fn begin() {
    run().await.unwrap()
}
