use crate::generated::rootfs::populate_rootfs;
use anyhow::Result;
use vfs::{MemoryFS, VfsPath};

pub fn get_root() -> Result<VfsPath> {
    let mut path: VfsPath = MemoryFS::new().into();
    populate_rootfs(&mut path)?;
    assert!(path.join("usr").unwrap().exists().unwrap());
    Ok(path)
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
