use libc::c_int;

use futures::prelude::*;
use tokio::signal::unix::SignalKind;

pub fn create() -> Result<impl Stream<Item = c_int> + Unpin, std::io::Error> {
    let out = stream::empty();

    let out = stream::select(out, single_signal_stream(libc::SIGALRM)?);
    let out = stream::select(out, single_signal_stream(libc::SIGCHLD)?);
    let out = stream::select(out, single_signal_stream(libc::SIGHUP)?);
    let out = stream::select(out, single_signal_stream(libc::SIGINT)?);
    let out = stream::select(out, single_signal_stream(libc::SIGIO)?);
    let out = stream::select(out, single_signal_stream(libc::SIGPIPE)?);
    let out = stream::select(out, single_signal_stream(libc::SIGQUIT)?);
    let out = stream::select(out, single_signal_stream(libc::SIGTERM)?);
    let out = stream::select(out, single_signal_stream(libc::SIGUSR1)?);
    let out = stream::select(out, single_signal_stream(libc::SIGUSR2)?);
    let out = stream::select(out, single_signal_stream(libc::SIGWINCH)?);

    Ok(out)
}

fn single_signal_stream(sig: c_int) -> Result<impl Stream<Item = c_int> + Unpin, std::io::Error> {
    Ok(tokio::signal::unix::signal(SignalKind::from_raw(sig))?.map(move |_| sig))
}
