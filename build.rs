use anyhow::Result;
use itertools::Itertools;
use std::{
    env,
    fs::File,
    io::{Read, Write},
    process::Command,
};
use walkdir::WalkDir;

const ROOTFS_RS_PATH: &str = "./src/generated/rootfs.rs";

/// Format a rust file.
fn format_file(path: &str) -> Result<()> {
    Command::new("rustfmt").arg(path).status()?;
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
            writeln!(&mut file, "    path.join(\"{path}\")?.create_dir()?;")?;
        } else {
            let mut contents = Vec::new();
            File::open(path)?.read_to_end(&mut contents)?;
            let path = path.display();
            writeln!(
                &mut file,
                "    let mut file = path.join(\"{path}\")?.create_file()?;"
            )?;
            if !contents.is_empty() {
                writeln!(&mut file, "    file.write_all(&[")?;
                for chunk in contents.chunks(15) {
                    write!(&mut file, "        ")?;
                    // TODO: Use std vesion when this is in stable
                    #[allow(unstable_name_collisions)]
                    let chunk = chunk
                        .iter()
                        .map(|byte| format!("0x{byte:02x},"))
                        .intersperse(String::from(" "));
                    for string in chunk {
                        write!(&mut file, "{string}")?;
                    }
                    writeln!(&mut file)?;
                }
                writeln!(&mut file, "    ])?;\n")?;
            }
        }
    }

    writeln!(&mut file, "    Ok(())\n}}")?;

    Ok(())
}

fn main() -> Result<()> {
    // Tell cargo to invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed=rootfs");

    generate_rootfs_rs()?;
    format_file(ROOTFS_RS_PATH)
}
