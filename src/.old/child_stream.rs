use std::sync::Arc;
use std::process;
use futures::{Future, Stream};
use futures::future;
use tokio_process::CommandExt;
use tokio_core::reactor::Handle;

use super::Event;


pub fn create(program: &String, args: &Vec<String>, core_handle: &Handle) -> Box<Stream<Item=Event,Error=()>> {
    let mut cmd = process::Command::new(program);
    cmd.args(args);

    match cmd.spawn_async(core_handle) {
        Err(io_error) => {
            let io_error_event = Event::IOError(Arc::new(io_error));
            let io_error_future = future::ok::<Event, ()>(io_error_event);
            Box::new(io_error_future.into_stream())
        },
        Ok(child) => {
            let child_pid = child.id();
            let child_born_event = Event::ChildBorn(child_pid);
            let child_born_future = future::ok::<Event, ()>(child_born_event);
            child.forget();
            debug!("Started child [prog={:?}; args={:?}; child_pid={:?}]", program, args, child_pid);

            Box::new(child_born_future.into_stream())
        },
    }
}
