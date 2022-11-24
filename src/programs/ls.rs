use crate::process::Process;
use crate::programs::common::color_picker::{Color, ColorPicker};
use anyhow::{bail, Result};

const DIR_COLOR: Color = Color::Blue;

pub async fn ls(process: &mut Process, args: Vec<String>) -> Result<()> {
    let path = if args.len() == 1 {
        process.cwd.clone()
    } else {
        let dir = process.get_path(&args[1])?;
        if !dir.exists()? {
            bail!("ls: no such file or directory: {}", dir.as_str());
        }
        if !dir.is_dir()? {
            bail!("ls: not a directory: {}", dir.as_str());
        }
        dir
    };

    let mut picker = ColorPicker::new(process.stdout.to_terminal().await?);

    for entity in path.walk_dir()? {
        let entity = entity?;
        if entity.parent().as_ref() != Some(&path) {
            break;
        }

        if entity.is_dir()? {
            picker.set_color(DIR_COLOR);
        } else {
            picker.reset();
        }
        let display = format!("{}\n", entity.filename());
        picker.write(&mut process.stdout, &display)?;
    }
    Ok(())
}
