use anyhow::{anyhow, Result};
use std::{
    env,
    fs::{self, File},
    io::{Read, Write},
    process::Command,
};
use walkdir::WalkDir;

const ROOTFS_RS_PATH: &str = "./src/generated/rootfs.rs";

/// Format a rust file.
fn format_file(path: &str) -> Result<()> {
    let status = Command::new("rustfmt").arg(path).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("Rustfmt failed"))
    }
}

fn generate_bin_directory() -> Result<()> {
    env::set_current_dir("src/programs")?;

    // Get list of all programs.
    let mut bins: Vec<String> = Vec::new();
    for entry in fs::read_dir("./")? {
        let path = entry?.path().display().to_string();
        let path = path.replace("./", "");
        if path.starts_with('.') || path == "mod.rs" || path == "common" {
            continue;
        }
        bins.push(path.replace(".rs", ""));
    }

    env::set_current_dir("../../rootfs/bin/")?;

    // Write files to bin directory
    for bin in bins {
        println!("{bin}");
        let mut file = File::create(&bin).unwrap();
        file.write_all(b"#!sh\n")?;
        file.write_all(b"# This is an internal command.\n")?;
        file.write_all(format!("{bin} ${{@}}\n").as_bytes())?;
    }

    env::set_current_dir("../..")?;
    Ok(())
}

fn generate_rootfs_rs() -> Result<()> {
    let mut file = File::create(ROOTFS_RS_PATH).unwrap();
    writeln!(
        &mut file,
        "// @generated
#![allow(unused)]
use anyhow::Result;
use vfs::VfsPath;
pub fn populate_rootfs(path: &mut VfsPath) -> Result<()> {{"
    )?;
    env::set_current_dir("rootfs")?;
    for entry in WalkDir::new(".").into_iter().skip(1).filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            let path = path.display();
            writeln!(&mut file, "path.join(\"{path}\")?.create_dir()?;")?;
        } else {
            // Ignore meta files
            let file_name = path.file_name().unwrap();
            if file_name == ".gitignore" || file_name == ".placeholder" {
                continue;
            }

            let mut contents = Vec::new();
            File::open(path)?.read_to_end(&mut contents)?;
            let path = path.display();

            writeln!(
                &mut file,
                "let mut file = path.join(\"{path}\")?.create_file()?;"
            )?;
            if !contents.is_empty() {
                writeln!(&mut file, "file.write_all(&[")?;
                let contents: String = contents
                    .iter()
                    .map(|byte| format!("0x{byte:02x},"))
                    .collect();
                writeln!(&mut file, "{contents}])?;")?;
            }
        }
    }
    writeln!(&mut file, "Ok(())}}")?;

    env::set_current_dir("..")?;

    Ok(())
}

fn main() -> Result<()> {
    // Tell cargo to invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed=rootfs");

    generate_bin_directory()?;
    generate_rootfs_rs()?;

    format_file(ROOTFS_RS_PATH)
}
