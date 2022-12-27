use crate::generated::rootfs::populate_rootfs;
use anyhow::Result;
use vfs::{MemoryFS, VfsPath};

/// Get the RootFS as a VfsPath.
pub fn get_root() -> Result<VfsPath> {
    let mut path: VfsPath = MemoryFS::new().into();
    populate_rootfs(&mut path)?;
    debug_assert!(path.join("usr").unwrap().exists().unwrap());
    Ok(path)
}

/// Convert VfsPath to str
pub fn vfs_path_to_str(path: &VfsPath) -> &str {
    let path = path.as_str();
    if path.is_empty() {
        "/"
    } else {
        path
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn embedded_fs() {
        let path = get_root().unwrap();

        assert!(path.exists().unwrap());
        assert!(path.join("usr").unwrap().exists().unwrap());
        assert!(path
            .join("usr/share/games/fortunes")
            .unwrap()
            .exists()
            .unwrap());
    }
}
