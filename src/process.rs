use crate::streams::InputStream;
use crate::streams::OutputStream;
use anyhow::Result;
use std::collections::HashSet;
use std::future::Future;

#[derive(Clone)]
pub struct Process {
    pub stdin: InputStream,
    pub stdout: OutputStream,
    pub env: HashSet<String, String>,
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
