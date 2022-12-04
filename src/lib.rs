mod ansi_codes;
mod filesystem;
mod generated;
mod process;
mod programs;
mod streams;
mod utils;
use ansi_codes::AnsiCode;
use anyhow::Result;
use futures::try_join;
use process::Process;
use wasm_bindgen::prelude::*;

const DEFAULT_SEARCH_PATH: &str = "bin";

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
    };
    shell.env.insert("PATH".into(), DEFAULT_SEARCH_PATH.into());

    try_join!(backend.run(), async {
        programs::shell(&mut shell, Default::default()).await?;
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
