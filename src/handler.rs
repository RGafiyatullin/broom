use super::*;

use std::collections::HashSet;
use tokio::signal::unix::SignalKind;

use child_process::ProcessEvent;


#[derive(Debug)]
pub struct Handler {
    immediate_children: HashSet<u32>,
}

impl Handler {
    pub fn new() -> Self {
        Self {
            immediate_children: Default::default(),
        }
    }

    pub async fn handle(&mut self, event: Event) -> Result<bool, ::failure::Error> {
        log::trace!("EVENT: {:#?}", event);

        match event {
            Event::ProcessEvent(ProcessEvent::Started {
                pid,
                mut stdin,
                mut stdout,
                mut stderr,
            }) => {
                let _ = self.immediate_children.insert(pid);

                let mut own_stdin = tokio::io::stdin();
                let _stdin_done = tokio::spawn(async move { tokio::io::copy(&mut own_stdin, &mut stdin).await });

                let mut own_stdout = tokio::io::stdout();
                let _stdout_done = tokio::spawn(async move { tokio::io::copy(&mut stdout, &mut own_stdout).await });

                let mut own_stderr = tokio::io::stderr();
                let _stderr_done = tokio::spawn(async move { tokio::io::copy(&mut stderr, &mut own_stderr).await });

                Ok(true)
            },

            Event::ProcessEvent(ProcessEvent::Terminated {
                pid,
                result: Err(reason),
            }) =>
                Err(reason.into()),

            Event::ProcessEvent(ProcessEvent::Terminated {
                pid,
                result: Ok(exit_status),
            }) => {
                let _ = self.immediate_children.remove(&pid);

                ::log::warn!("REAP REMAINING CHILDREN");
                ::log::warn!("PAY ATTENTION TO THE exit_status: {:#?}", exit_status);

                Ok(!self.immediate_children.is_empty())
            },

            Event::Signal(sig) => {
                if sig == ::libc::SIGCHLD {
                    ::log::warn!("REAP SINGLE CHILD");
                    Ok(true)
                } else {
                    for pid in &self.immediate_children {
                        ::log::trace!("forwarding sig[{}] to pid[{}]...", sig, pid);
                        unsafe { ::libc::kill(*pid as i32, sig); }
                    }
                    Ok(true)
                }
            }, 

            unexpected => Err(::failure::format_err!(
                "Unexpected Event: {:#?}",
                unexpected
            )),
        }
    }
}


