
use std::io;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum Event {
    ChildBorn(u32),
    // ChildDied(u32, i32),
    Signal(i32),
    TerminationTimeout,
    IOError(Arc<io::Error>),
}
