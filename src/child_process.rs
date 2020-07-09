use std::collections::HashMap;
use std::io;
use std::process::ExitStatus;
use std::process::Stdio;

use futures::prelude::*;
use tokio::process::ChildStderr;
use tokio::process::ChildStdin;
use tokio::process::ChildStdout;
use tokio::process::Command;

#[derive(Debug)]
pub enum ProcessEvent {
    Started {
        pid: u32,
        stdin: ChildStdin,
        stdout: ChildStdout,
        stderr: ChildStderr,
    },
}

#[derive(Debug, Clone)]
pub struct ProcessArgs {
    exec: String,
    args: Vec<String>,
    env: HashMap<String, String>,
}

impl ProcessArgs {
    pub fn new<S: AsRef<str>>(exec: S) -> Self {
        let exec = exec.as_ref().to_owned();
        Self {
            exec,
            args: Default::default(),
            env: Default::default(),
        }
    }
    pub fn with_args<A: IntoIterator<Item = impl AsRef<str>>>(self, args: A) -> Self {
        let args = args.into_iter().map(|s| s.as_ref().to_owned()).collect();
        Self { args, ..self }
    }

    pub fn with_env<E: IntoIterator<Item = (impl AsRef<str>, impl AsRef<str>)>>(
        self,
        env: E,
    ) -> Self {
        let env = env
            .into_iter()
            .map(|(k, v)| (k.as_ref().to_owned(), v.as_ref().to_owned()))
            .collect();
        Self { env, ..self }
    }

    pub fn start(&self) -> Result<impl Stream<Item = ProcessEvent>, ::failure::Error> {
        let mut cmd = Command::new(&self.exec);
        cmd.args(&self.args)
            .envs(&self.env)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped());

        let mut child = cmd.spawn()?;

        let pid = child.id();
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ::failure::format_err!("failed to take stdout"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| ::failure::format_err!("failed to take stderr"))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| ::failure::format_err!("failed to take stdin"))?;

        let start_event = ProcessEvent::Started {
            pid,
            stdout,
            stderr,
            stdin,
        };

        let start_event = stream::once(future::ready(start_event));

        Ok(start_event)
    }
}
