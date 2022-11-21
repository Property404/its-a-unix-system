use crate::streams::{InputStream, OutputStream};
use anyhow::Result;
use futures::channel::{mpsc::UnboundedSender, oneshot};
use std::{collections::HashSet, future::Future};
use vfs::VfsPath;

#[derive(Clone)]
pub struct Process {
    pub stdin: InputStream,
    pub stdout: OutputStream,
    pub env: HashSet<String, String>,
    pub cwd: VfsPath,
    // Used by a process to indicate it's listening for signals
    // Currently we just have the ^C signal, but we might add more
    // later. I don't know, I'm tired
    pub signal_registrar: UnboundedSender<oneshot::Sender<()>>,
}

impl Process {
    pub async fn run<'a, Fut>(
        &'a mut self,
        f: impl FnOnce(&'a mut Process, Vec<String>) -> Fut,
        args: Vec<String>,
    ) -> Result<()>
    where
        Fut: Future<Output = Result<()>> + 'a,
    {
        f(self, args).await
    }
}
