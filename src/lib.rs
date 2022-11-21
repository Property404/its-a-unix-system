mod process;
mod programs;
mod streams;
mod utils;
use anyhow::Result;
use futures::try_join;
use process::Process;

use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

async fn run() -> Result<()> {
    utils::set_panic_hook();

    let (mut stdin, stdout, mut backend, signal_registrar) = streams::standard()?;

    let mut shell = Process {
        stdin: stdin.clone(),
        stdout: stdout.clone(),
        env: Default::default(),
        signal_registrar,
    };

    try_join!(backend.run(), async {
        shell.run(programs::shell, Default::default()).await?;
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
