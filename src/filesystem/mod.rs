use crate::generated::rootfs::populate_rootfs;
use anyhow::Result;
use std::{collections::HashMap, io};
use vfs::{MemoryFS, VfsPath};
mod dev;
mod multi;
use dev::{Device, DeviceFS};
use multi::MultiFS;

// `/dev/null` implementation
#[derive(Debug)]
struct NullDevice {}

impl io::Write for NullDevice {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl io::Seek for NullDevice {
    fn seek(&mut self, _pos: io::SeekFrom) -> io::Result<u64> {
        Ok(0)
    }
}

impl io::Read for NullDevice {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        Ok(0)
    }
}

impl Device for NullDevice {
    fn clone_box(&self) -> Box<dyn Device> {
        Box::new(NullDevice {})
    }
}

/// Get the RootFS as a VfsPath.
pub fn get_root() -> Result<VfsPath> {
    let mut memfs: VfsPath = MemoryFS::new().into();
    populate_rootfs(&mut memfs)?;

    let mut devices: HashMap<String, Box<dyn Device>> = HashMap::new();
    devices.insert(String::from("/null"), Box::new(NullDevice {}));
    let devfs: VfsPath = DeviceFS::new(devices).into();

    let mut root = MultiFS::new(memfs)?;
    root.push("/dev", devfs)?;

    let root: VfsPath = root.into();
    debug_assert!(root.join("/usr").unwrap().exists().unwrap());
    debug_assert!(root.join("/usr/share").unwrap().exists().unwrap());
    debug_assert!(root.join("/bin/fortune").unwrap().exists().unwrap());
    debug_assert!(root.join("/dev").unwrap().exists().unwrap());
    debug_assert!(root.join("/dev/null").unwrap().exists().unwrap());

    Ok(root)
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
