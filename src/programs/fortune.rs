use crate::process::Process;
use anyhow::{bail, Result};
use clap::Parser;
use rand::seq::SliceRandom;
use std::io::Write;

const NORMAL_FORTUNES: &str = "/usr/share/games/fortunes/fortunes";
const RISQUE_FORTUNES: &str = "/usr/share/games/fortunes/risque";
const REPEAT_FILE: &str = "/run/fortunes.history";

/// Generate a fortune, quote, or wise adage.
#[derive(Parser)]
struct Options {
    /// Only show short fortunes.
    #[arg(short)]
    short: bool,
    /// Include risqué fortunes.
    #[arg(short = 'r')]
    risque: bool,
}

pub async fn fortune(process: &mut Process, args: Vec<String>) -> Result<()> {
    let options = Options::try_parse_from(args.into_iter())?;
    // We keep a history of past fortunes so we don't repeat too often.
    let (fortunes_told, mut repeat_file) = {
        let path = process.get_path(REPEAT_FILE)?;
        let mut repeat_file = {
            if let Ok(file) = path.open_file() {
                file
            } else {
                path.create_file()?;
                path.open_file()?
            }
        };
        let mut fortunes_told = String::new();
        repeat_file.read_to_string(&mut fortunes_told)?;
        let repeat_file = path.append_file()?;
        (fortunes_told, repeat_file)
    };

    let mut fortunes = String::new();
    let mut file = process.get_path(NORMAL_FORTUNES)?.open_file()?;
    file.read_to_string(&mut fortunes)?;
    if options.risque {
        let mut file = process.get_path(RISQUE_FORTUNES)?.open_file()?;
        fortunes.push('\n');
        file.read_to_string(&mut fortunes)?;
    }
    let fortunes = fortunes.trim().split("\n\n").collect::<Vec<&str>>();

    let mut loops = 0;
    loop {
        let Some(fortune) = fortunes.choose(&mut rand::thread_rng()) else {
            bail!("fortune: Could not select a fortune");
        };

        // Try to not give a fortune already given.
        if (loops < 5 && fortunes_told.contains(fortune)) ||
            // Only give short fortunes if requested.
            (options.short && fortune.len() >= 80)
        {
            loops += 1;
            continue;
        }

        process.stdout.write_all(fortune.as_bytes())?;
        process.stdout.write_all(b"\n")?;
        repeat_file.write_all(fortune.as_bytes())?;
        repeat_file.write_all(b"\n")?;
        return Ok(());
    }
}
