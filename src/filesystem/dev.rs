//! A filesystem for "files" with custom behavior.

use std::{collections::HashMap, io::Write};
use vfs::{
    error::VfsErrorKind,
    {FileSystem, SeekAndRead, VfsFileType, VfsMetadata, VfsResult},
};

pub trait Device: Write + SeekAndRead + Send + Sync + std::fmt::Debug {
    fn metadata(&self) -> VfsMetadata {
        VfsMetadata {
            file_type: VfsFileType::File,
            len: 0,
        }
    }
    fn clone_box(&self) -> Box<dyn Device>;
}

/// A filesytem that allows custom read/write behavior for individual files.
#[derive(Debug)]
pub struct DeviceFS {
    devices: HashMap<String, Box<dyn Device>>,
}

impl DeviceFS {
    /// Create a new DeviceFS with the given devices.
    pub fn new(devices: HashMap<String, Box<dyn Device>>) -> Self {
        DeviceFS { devices }
    }

    fn get_device(&self, path: &str) -> VfsResult<Box<dyn Device>> {
        Ok(self
            .devices
            .get(path)
            .ok_or(VfsErrorKind::FileNotFound)?
            .clone_box())
    }
}

impl FileSystem for DeviceFS {
    fn read_dir(&self, path: &str) -> VfsResult<Box<dyn Iterator<Item = String> + Send>> {
        #[allow(clippy::needless_collect)]
        let entries: Vec<_> = self
            .devices
            .iter()
            .filter_map(|tuple| {
                if tuple.0.starts_with(path) {
                    Some(tuple.0.clone())
                } else {
                    None
                }
            })
            .collect();

        Ok(Box::new(entries.into_iter()))
    }

    fn create_dir(&self, _path: &str) -> VfsResult<()> {
        Err(VfsErrorKind::Other("Unimplemented".into()).into())
    }

    fn open_file(&self, path: &str) -> VfsResult<Box<dyn SeekAndRead + Send>> {
        Ok(Box::new(self.get_device(path)?))
    }

    fn create_file(&self, path: &str) -> VfsResult<Box<dyn Write + Send>> {
        Ok(Box::new(self.get_device(path)?))
    }

    fn append_file(&self, path: &str) -> VfsResult<Box<dyn Write + Send>> {
        Ok(Box::new(self.get_device(path)?))
    }

    fn metadata(&self, path: &str) -> VfsResult<VfsMetadata> {
        if path.is_empty() {
            return Ok(VfsMetadata {
                file_type: VfsFileType::Directory,
                len: 0,
            });
        }
        Ok(self.get_device(path)?.metadata())
    }

    fn exists(&self, path: &str) -> VfsResult<bool> {
        Ok(path.is_empty() || self.get_device(path).is_ok())
    }

    fn remove_file(&self, _path: &str) -> VfsResult<()> {
        Err(VfsErrorKind::Other("Unimplemented".into()).into())
    }

    fn remove_dir(&self, _path: &str) -> VfsResult<()> {
        Err(VfsErrorKind::Other("Unimplemented".into()).into())
    }
}
