use anyhow::Result;
use std::{
    env,
    fs::File,
    io::{Read, Write},
};
use walkdir::WalkDir;

fn main() -> Result<()> {
    // Tell cargo to invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed=rootfs");
    let mut file = File::create("./src/generated/rootfs.rs").unwrap();
    writeln!(
        &mut file,
        "
use anyhow::Result;
use vfs::VfsPath;

pub fn populate_rootfs(path: &mut VfsPath) -> Result<()> {{"
    )?;
    env::set_current_dir("rootfs")?;
    for entry in WalkDir::new(".").into_iter().skip(1).filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            let path = path.display();
            writeln!(&mut file, "\tpath.join(\"{path}\")?.create_dir()?;")?;
        } else {
            let mut contents = Vec::new();
            File::open(path)?.read_to_end(&mut contents)?;
            let path = path.display();
            writeln!(
                &mut file,
                "\t let mut file = path.join(\"{path}\")?.create_file()?;"
            )?;
            write!(&mut file, "\t file.write_all(&[")?;
            for byte in contents {
                write!(&mut file, "0x{:0x},", byte)?;
            }
            writeln!(&mut file, "])?;\n")?;
        }
    }

    writeln!(&mut file, "Ok(())\n}}")?;

    Ok(())
}
