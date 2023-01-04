//! An branching file system combining two or more filesystems.
use std::{collections::HashSet, io::Write};
use vfs::{
    error::VfsErrorKind,
    {FileSystem, SeekAndRead, VfsMetadata, VfsPath, VfsResult},
};

// check if `ancestor` is an ancestor of `child`
fn contains(ancestor: &VfsPath, child: &VfsPath) -> bool {
    let mut child = child.clone();
    while !child.is_root() {
        child = child.parent();
        if &child == ancestor {
            return true;
        }
    }
    false
}

/// An file system combining several filesystems into one.
///
/// Each layer is a branch of the root path.
#[derive(Debug, Clone)]
pub struct MultiFS {
    root: VfsPath,
    layers: Vec<(VfsPath, VfsPath)>,
}

impl MultiFS {
    /// Create a new MultiFS filesystem from the given root.
    pub fn new(root: VfsPath) -> VfsResult<Self> {
        if !root.is_root() {
            return Err(VfsErrorKind::Other("Root is not a root path".into()).into());
        }

        Ok(MultiFS {
            layers: [(root.clone(), root.clone())].into(),
            root,
        })
    }

    /// Append a new filesystem `local_root` coming out of path `path`, relative to root.
    pub fn push(&mut self, path: &str, local_root: VfsPath) -> VfsResult<()> {
        let path = self.root.join(path)?;

        if !local_root.is_root() {
            return Err(VfsErrorKind::Other("local root is not a root path".into()).into());
        }

        self.layers.push((path, local_root));
        self.layers
            .sort_by(|a, b| b.0.as_str().len().cmp(&a.0.as_str().len()));
        Ok(())
    }

    fn get_path(&self, mut path_str: &str) -> VfsResult<VfsPath> {
        if path_str.ends_with('/') {
            path_str = &path_str[0..path_str.len() - 1];
        }

        let path = self.root.join(path_str)?;
        for (layer, local_root) in &self.layers {
            if layer == &path {
                return Ok(local_root.clone());
            }
            if contains(layer, &path) {
                let path_str = &path_str[layer.as_str().len()..];
                return local_root.join(path_str);
            }
        }
        Err(VfsErrorKind::FileNotFound.into())
    }

    fn ensure_has_parent(&self, path: &str) -> VfsResult<()> {
        let separator = path.rfind('/');
        if let Some(index) = separator {
            let parent_path = &path[..index];
            if self.exists(parent_path)? {
                self.get_path(parent_path)?.create_dir_all()?;
                return Ok(());
            }
        }
        Err(VfsErrorKind::Other("!Parent path does not exist".into()).into())
    }
}

impl FileSystem for MultiFS {
    fn read_dir(&self, path: &str) -> VfsResult<Box<dyn Iterator<Item = String> + Send>> {
        let path = self.get_path(path)?;

        let mut entries = HashSet::<String>::new();
        for path in path.read_dir()? {
            entries.insert(path.filename());
        }

        for (layer, _) in &self.layers {
            if layer.parent() == path && !layer.is_root() {
                entries.insert(layer.filename());
            }
        }

        Ok(Box::new(entries.into_iter()))
    }

    fn create_dir(&self, path: &str) -> VfsResult<()> {
        self.get_path(path)?.create_dir_all()?;
        Ok(())
    }

    fn open_file(&self, path: &str) -> VfsResult<Box<dyn SeekAndRead + Send>> {
        self.get_path(path)?.open_file()
    }

    fn create_file(&self, path: &str) -> VfsResult<Box<dyn Write + Send>> {
        self.ensure_has_parent(path)?;
        let result = self.get_path(path)?.create_file()?;
        Ok(result)
    }

    fn append_file(&self, path: &str) -> VfsResult<Box<dyn Write + Send>> {
        let write_path = self.get_path(path)?;
        if !write_path.exists()? {
            self.ensure_has_parent(path)?;
            self.get_path(path)?.copy_file(&write_path)?;
        }
        write_path.append_file()
    }

    fn metadata(&self, path: &str) -> VfsResult<VfsMetadata> {
        self.get_path(path)?.metadata()
    }

    fn exists(&self, path: &str) -> VfsResult<bool> {
        self.get_path(path)?.exists()
    }

    fn remove_file(&self, path: &str) -> VfsResult<()> {
        let write_path = self.get_path(path)?;
        if write_path.exists()? {
            write_path.remove_file()?;
        }
        Ok(())
    }

    fn remove_dir(&self, path: &str) -> VfsResult<()> {
        let write_path = self.get_path(path)?;
        if write_path.exists()? {
            write_path.remove_dir()?;
        }
        Ok(())
    }
}
