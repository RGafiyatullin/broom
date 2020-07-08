
use tokio_core::reactor::Handle;
use tokio_signal::unix;
use futures::{Future, Stream};
use futures::future;
use std::sync::Arc;

use super::Event;

pub fn create(signal: i32, core_handle: &Handle) -> Box<Stream<Item=Event,Error=()>> {
    Box::new(
        unix::Signal::new(signal, core_handle)
        .wait()
        .expect("Failed to install handler")
        .map(move |_|
            Event::Signal(signal))
        .or_else(|io_err| {
            future::ok::<Event, ()>(Event::IOError(Arc::new(io_err)))
        })
    )
}
