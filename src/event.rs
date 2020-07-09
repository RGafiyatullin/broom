use tokio::signal::unix::SignalKind;
use libc::c_int;

use crate::child_process::ProcessEvent;

#[derive(Debug)]
pub enum Event {
    ProcessEvent(ProcessEvent),
    Signal(c_int),
}
