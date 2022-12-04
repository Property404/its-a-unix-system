use crate::process::Process;
use crate::programs::common::color_picker::{Color, ColorPicker};
use anyhow::{bail, Result};
use clap::Parser;

const DIR_COLOR: Color = Color::Blue;

/// List files/directories.
#[derive(Parser)]
struct Options {
    /// Do not ignore hidden files.
    #[arg(short, long)]
    all: bool,
    /// The directory to list.
    target: Option<String>,
}

pub async fn ls(process: &mut Process, args: Vec<String>) -> Result<()> {
    let options = Options::try_parse_from(args.into_iter())?;
    let path = {
        let dir = process.get_path(options.target.unwrap_or_else(|| ".".into()))?;
        if !dir.exists()? {
            bail!("No such file or directory: {}", dir.as_str());
        }
        if !dir.is_dir()? {
            bail!("Not a directory: {}", dir.as_str());
        }
        dir
    };

    let mut picker = ColorPicker::new(process.stdout.to_terminal().await?);

    // Show '.' and '..', because those don't appear in walk_dir()
    if options.all {
        picker.set_color(DIR_COLOR);
        picker.write(&mut process.stdout, ".\n..\n")?;
    }

    for entity in path.read_dir()? {
        if entity.is_dir()? {
            picker.set_color(DIR_COLOR);
        } else {
            picker.reset();
        }
        let display = format!("{}\n", entity.filename());
        if options.all || !display.starts_with('.') {
            picker.write(&mut process.stdout, &display)?;
        }
    }
    Ok(())
}
