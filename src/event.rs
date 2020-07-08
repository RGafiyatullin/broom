use std::process::ExitStatus;

#[derive(Debug, Clone)]
pub enum Event {
    ChildBorn(u32),
    ChildExit(ExitStatus),
    ChildTerm,

    ChildSignal,
    ShutdownRequest,
}
