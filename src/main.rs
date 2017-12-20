
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate libc;
extern crate futures;
extern crate tokio_core;
extern crate tokio_signal;
extern crate tokio_process;

mod signal_stream;
mod child_stream;
mod event;
mod event_dispatcher;

use std::{env, time};
use futures::Stream;
use futures::future;
use futures::sync::mpsc;
use tokio_core::reactor::{Core, Timeout};
use tokio_signal::unix;

use event::Event;

fn args() -> (String, Vec<String>) {
    let args: Vec<String> = env::args().collect();
    let args_len = args.len();
    let argv0 = &args[1];
    let argv = &args[2..args_len];

    (argv0.clone(), argv.to_vec())
}

fn opts_termination_timeout_duration() -> time::Duration {
    let d = env::var("BROOM_TERMINATION_TIMEOUT_SEC")
        .unwrap_or(String::from("10"))
        .parse().unwrap();
    let d = time::Duration::from_secs(d);
    debug!("Graceful child termination timeout: {:?}", d);
    d
}

fn main() {
    env_logger::init().unwrap();

    let mut core = Core::new().unwrap();
    let core_handle = &core.handle();

    let termination_timeout_duration = opts_termination_timeout_duration();
    let (argv0, argv) = args();

    let (termination_timeout_source, termination_timeout_stream) = mpsc::channel::<()>(1);
    let termination_timeout_stream = termination_timeout_stream
        .then(|_|
            future::result(Timeout::new(termination_timeout_duration, core_handle)))
        .and_then(|timeout| {
            debug!("Initiated termination timeout alarm [{:?}]", termination_timeout_duration);
            timeout
        })
        .map(|_| {
            debug!("Termination timer done. Emitting Event::TerminationTimeout");
            Event::TerminationTimeout
        })
        .map_err(|_| ());

    let stream_sigterm = signal_stream::create(unix::libc::SIGTERM, core_handle);
    let stream_sigint = signal_stream::create(unix::libc::SIGINT, core_handle);
    let stream_sigchld = signal_stream::create(unix::libc::SIGCHLD, core_handle);

    let stream_child = child_stream::create(&argv0, &argv, core_handle);

    let stream =
        stream_child
        .select(stream_sigterm)
        .select(stream_sigint)
        .select(stream_sigchld)
        .select(termination_timeout_stream)
        .fold(event_dispatcher::new(termination_timeout_source), |acc, event| acc.handle(event));

    core.run(stream).expect("Failed to run stream");
}
