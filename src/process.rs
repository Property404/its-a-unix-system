use crate::streams::{InputStream, OutputStream};
use anyhow::Result;
use futures::channel::{mpsc::UnboundedSender, oneshot};
use std::collections::HashSet;
use vfs::VfsPath;

#[derive(Clone)]
pub struct Process {
    pub stdin: InputStream,
    pub stdout: OutputStream,
    pub stderr: OutputStream,
    pub env: HashSet<String, String>,
    pub cwd: VfsPath,
    // Used by a process to indicate it's listening for signals
    // Currently we just have the ^C signal, but we might add more
    // later. I don't know, I'm tired
    pub signal_registrar: UnboundedSender<oneshot::Sender<()>>,
}

impl Process {
    /// Get VFS path from , as you would type into a Unix shell.
    pub fn get_path(&self, path: impl AsRef<str>) -> Result<VfsPath> {
        let mut root = false;
        let mut path = path.as_ref();
        while path.starts_with('/') {
            path = &path[1..];
            root = true;
        }
        if path.ends_with('/') {
            path = &path[0..path.len() - 1]
        }

        Ok(if root {
            self.cwd.root().join(path)
        } else {
            self.cwd.join(path)
        }?)
    }
}
