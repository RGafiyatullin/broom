use super::*;

use std::collections::HashSet;

use ::futures::channel::oneshot;
use ::futures::prelude::*;

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
                let stdin_copying =
                    async move { tokio::io::copy(&mut own_stdin, &mut stdin).await }
                        .map_err(::failure::Error::from);

                let _stdin_done = tokio::spawn(stdin_copying).map_err(::failure::Error::from);

                let mut own_stdout = tokio::io::stdout();
                let stdout_copying =
                    async move { tokio::io::copy(&mut stdout, &mut own_stdout).await }
                        .map_ok(|bc| log::trace!("stdout complete [bc: {}]", bc))
                        .map_err(::failure::Error::from);
                let stdout_done = tokio::spawn(stdout_copying)
                    .map_err(::failure::Error::from)
                    .and_then(future::ready);

                let mut own_stderr = tokio::io::stderr();
                let stderr_copying =
                    async move { tokio::io::copy(&mut stderr, &mut own_stderr).await }
                        .map_ok(|bc| log::trace!("stderr complete [bc: {}]", bc))
                        .map_err(::failure::Error::from);
                let stderr_done = tokio::spawn(stderr_copying)
                    .map_err(::failure::Error::from)
                    .and_then(future::ready);

                let on_child_outputs_done_cancel_stdin_copying =
                    future::try_join(stdout_done, stderr_done).and_then(move |_| async {
                        // XXX: well stdin is not geniunely asynchronous, so its copying cannot be cancelled by just not-polling it...
                        log::trace!("Terminating the current process");
                        let _ = std::process::exit(0);
                        Ok(())
                    });
                let _ = tokio::spawn(on_child_outputs_done_cancel_stdin_copying);

                Ok(true)
            }

            Event::Signal(sig) => {
                if sig == ::libc::SIGCHLD {
                    match wait_single_child()? {
                        WaitResult::Child(dead_child) => {
                            let was_an_immediate_child =
                                self.immediate_children.remove(&dead_child);
                            log::trace!(
                                "reaped a child [pid: {}; immediate: {:?}]",
                                dead_child,
                                was_an_immediate_child
                            );
                            if self.immediate_children.is_empty() {
                                let () = reap_remaining_children().await?;
                                Ok(false)
                            } else {
                                Ok(true)
                            }
                        }
                        WaitResult::NoChildren => {
                            log::trace!("No children left. Shutting down");
                            Ok(false)
                        }
                        WaitResult::NotReady => {
                            log::warn!("Received a SIGCHLD yet not child could be waited");
                            Ok(true)
                        }
                    }
                } else {
                    for pid in &self.immediate_children {
                        log::trace!("forwarding sig[{}] to pid[{}]...", sig, pid);
                        unsafe {
                            ::libc::kill(*pid as i32, sig);
                        }
                    }
                    Ok(true)
                }
            }
        }
    }
}

enum WaitResult {
    NotReady,
    NoChildren,
    Child(u32),
}

fn wait_single_child() -> Result<WaitResult, ::failure::Error> {
    let mut wait_status = 0;
    let waited_pid = unsafe { ::libc::waitpid(-1, &mut wait_status, libc::WNOHANG) };
    log::warn!(
        "wait_single_child => [pid: {}; status: {}]",
        waited_pid,
        wait_status
    );
    match waited_pid {
        -1 => Ok(WaitResult::NoChildren),
        0 => Ok(WaitResult::NotReady),
        dead_child_pid => Ok(WaitResult::Child(dead_child_pid as u32)),
    }
}

async fn reap_remaining_children() -> Result<(), ::failure::Error> {
    loop {
        use ::tokio::time;
        use std::time::Duration;

        match wait_single_child()? {
            WaitResult::Child(dead_child) => {
                log::trace!("Reaped an indirect child: {}", dead_child);
                continue;
            }
            WaitResult::NoChildren => {
                log::trace!("No children left");
                break;
            }
            WaitResult::NotReady => {
                let () = time::delay_for(Duration::from_millis(100)).await;
                continue;
            }
        }
    }
    Ok(())
}
