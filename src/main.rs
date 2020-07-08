mod event;
use event::Event;

mod handler;
use handler::Handler;

use std::collections::HashMap;
use std::env;
use std::time::Duration;

use futures::prelude::*;
use tokio::process::Command as OsCommand;
use tokio::signal::unix::signal as unix_signal;
use tokio::signal::unix::SignalKind as UnixSignalKind;

fn args() -> Result<(String, Vec<String>), ::failure::Error> {
    let args: Vec<String> = env::args().collect();
    let args_len = args.len();

    if args_len < 2 {
        Err(::failure::format_err!("Main child command unspecified"))?
    }

    let argv0 = &args[1];
    let argv = &args[2..args_len];

    Ok((argv0.clone(), argv.to_vec()))
}

fn env() -> HashMap<String, String> {
    std::env::vars().collect()
}

fn opts_termination_timeout_duration() -> Duration {
    let d = env::var("BROOM_TERMINATION_TIMEOUT_SEC")
        .unwrap_or(String::from("10"))
        .parse()
        .unwrap();
    let d = Duration::from_secs(d);
    log::debug!("Graceful child termination timeout: {:?}", d);
    d
}

async fn run() -> Result<(), ::failure::Error> {
    let signals = {
        let sigterm_stream =
            unix_signal(UnixSignalKind::terminate())?.map(|_| Event::ShutdownRequest);
        let sigint_stream =
            unix_signal(UnixSignalKind::interrupt())?.map(|_| Event::ShutdownRequest);
        let sigchld_stream = unix_signal(UnixSignalKind::child())?.map(|_| Event::ChildSignal);

        let stream = stream::select(sigterm_stream, sigint_stream);
        let stream = stream::select(stream, sigchld_stream);

        stream
    };

    let main_child = {
        use std::process::Stdio;

        let (cmd_exec, cmd_args) = args()?;
        let env_vars = env();

        let mut cmd = OsCommand::new(&cmd_exec);
        cmd.args(cmd_args)
            .env_clear()
            .envs(env_vars)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped());

        let mut spawned = cmd.spawn()?;

        let mut own_stdout = tokio::io::stdout();
        let mut child_stdout = spawned
            .stdout
            .take()
            .ok_or_else(|| ::failure::format_err!("Failed to take child stdout"))?;
        let stdout_redirected =
            async move { tokio::io::copy(&mut child_stdout, &mut own_stdout).await };
        let _ = tokio::spawn(stdout_redirected);

        let mut own_stderr = tokio::io::stderr();
        let mut child_stderr = spawned
            .stderr
            .take()
            .ok_or_else(|| ::failure::format_err!("Failed to take child stderr"))?;
        let stderr_redirected =
            async move { tokio::io::copy(&mut child_stderr, &mut own_stderr).await };
        let _ = tokio::spawn(stderr_redirected);

        let mut own_stdin = tokio::io::stdin();
        let mut child_stdin = spawned
            .stdin
            .take()
            .ok_or_else(|| ::failure::format_err!("Failed to take child stdin"))?;
        let stdin_redirected =
            async move { tokio::io::copy(&mut own_stdin, &mut child_stdin).await };
        let _ = tokio::spawn(stdin_redirected);

        log::info!("{:#?}", spawned);

        spawned
    };

    let child_born = stream::once(future::ready(Event::ChildBorn(main_child.id())));

    let events = child_born.chain(signals);

    let _ = events.fold(Handler::new(), Handler::handle).await;

    Ok(())
}

#[tokio::main]
async fn main() {
    let _ = dotenv::dotenv().ok();
    let () = env_logger::init();

    run().await.expect("Failure");

    // env_logger::init().unwrap();

    // let mut core = Core::new().unwrap();
    // let core_handle = &core.handle();

    // let termination_timeout_duration = opts_termination_timeout_duration();
    // let (argv0, argv) = args();

    // let (termination_timeout_source, termination_timeout_stream) = mpsc::channel::<()>(1);
    // let termination_timeout_stream = termination_timeout_stream
    //     .then(|_|
    //         future::result(Timeout::new(termination_timeout_duration, core_handle)))
    //     .and_then(|timeout| {
    //         debug!("Initiated termination timeout alarm [{:?}]", termination_timeout_duration);
    //         timeout
    //     })
    //     .map(|_| {
    //         debug!("Termination timer done. Emitting Event::TerminationTimeout");
    //         Event::TerminationTimeout
    //     })
    //     .map_err(|_| ());

    // let stream_sigterm = signal_stream::create(unix::libc::SIGTERM, core_handle);
    // let stream_sigint = signal_stream::create(unix::libc::SIGINT, core_handle);
    // let stream_sigchld = signal_stream::create(unix::libc::SIGCHLD, core_handle);

    // let stream_child = child_stream::create(&argv0, &argv, core_handle);

    // let stream =
    //     stream_child
    //     .select(stream_sigterm)
    //     .select(stream_sigint)
    //     .select(stream_sigchld)
    //     .select(termination_timeout_stream)
    //     .fold(event_dispatcher::new(termination_timeout_source), |acc, event| acc.handle(event));

    // core.run(stream).expect("Failed to run stream");
}
