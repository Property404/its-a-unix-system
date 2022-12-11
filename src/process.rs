use crate::streams::{InputStream, OutputStream};
use anyhow::Result;
use futures::channel::{mpsc::UnboundedSender, oneshot};
use std::collections::HashMap;
use vfs::VfsPath;

#[derive(Clone)]
pub struct Process {
    pub stdin: InputStream,
    pub stdout: OutputStream,
    pub stderr: OutputStream,
    pub env: HashMap<String, String>,
    pub cwd: VfsPath,
    pub args: Vec<String>,
    // Used by a process to indicate it's listening for signals
    // Currently we just have the ^C signal, but we might add more
    // later. I don't know, I'm tired
    pub signal_registrar: UnboundedSender<oneshot::Sender<()>>,
}

impl Process {
    /// Get VFS path from , as you would type into a Unix shell.
    pub fn get_path(&self, path: impl AsRef<str>) -> Result<VfsPath> {
        let mut path = path.as_ref();

        while path.len() > 1 && path.ends_with('/') {
            path = &path[0..path.len() - 1]
        }

        Ok(self.cwd.join(path)?)
    }
}
