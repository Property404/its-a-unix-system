use crate::{
    process::{ExitCode, Process},
    utils,
};
use anyhow::{bail, Result};
use clap::Parser;
use futures::io::AsyncWriteExt;

const THEMES_DIR: &str = "/usr/share/theme/themes";

/// Change the terminal theme.
///
/// Use without arguments to see available themes.
#[derive(Parser)]
#[command(verbatim_doc_comment)]
struct Options {
    /// The theme to switch to.
    theme: Option<String>,
}

pub async fn theme(process: &mut Process) -> Result<ExitCode> {
    let options = Options::try_parse_from(process.args.iter())?;

    let themes = process.get_path(THEMES_DIR)?;

    if let Some(theme) = &options.theme {
        let theme = themes.join(theme)?;
        let mut theme_contents = String::new();

        if !theme.exists()? {
            bail!("No such theme: {}", options.theme.expect("BUG: no theme"));
        }

        let mut theme = theme.open_file()?;
        theme.read_to_string(&mut theme_contents)?;

        let Some(theme_holder) = utils::get_document()?.get_element_by_id("theme-holder") else {
            bail!("Failed to get theme holder")
        };

        theme_holder.set_text_content(Some(&theme_contents));
    } else {
        process.stdout.write_all(b"Available themes:\n").await?;
        for theme in themes.read_dir()? {
            let theme = theme.filename();
            process.stdout.write_all(theme.as_bytes()).await?;
            process.stdout.write_all(b"\n").await?;
        }
    }

    Ok(ExitCode::SUCCESS)
}
