use crate::process::Process;
use anyhow::{bail, Result};
use rand::seq::SliceRandom;
use std::io::Write;

const FORTUNE_FILE: &str = "usr/share/games/fortunes";

pub async fn fortune(process: &mut Process, _args: Vec<String>) -> Result<()> {
    let mut fortunes = String::new();
    let mut file = process.cwd.root().join(FORTUNE_FILE)?.open_file()?;
    file.read_to_string(&mut fortunes)?;
    let fortunes = fortunes.trim().split("\n\n").collect::<Vec<&str>>();
    let Some(fortune) = fortunes.choose(&mut rand::thread_rng()) else {
        bail!("fortune: Could not select a fortune");
    };
    process.stdout.write_all(fortune.as_bytes())?;
    process.stdout.write_all(b"\n")?;
    Ok(())
}
