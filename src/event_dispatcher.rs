
use std::io;
use std::sync::Arc;
use std::process;
use std::{thread, time};
use std::collections::HashSet;
use futures::future;
use futures::sync::mpsc;
use futures::Future;
use tokio_signal::unix;

use libc;

use super::Event;

pub struct Dispatcher{
    termination_timeout_tx: mpsc::Sender<()>,
    immediate_children: HashSet<u32>,
}

impl Dispatcher {
    fn handle_child_born(mut self, child_pid: u32) -> Box<Future<Item=Self, Error=()>> {
        debug!("Child born [pid={:?}]", child_pid);
        self.immediate_children.insert(child_pid);
        Box::new(future::ok(self))
    }

    fn reap_remaining_children(&self) -> () {
        loop {
            let mut wait_status = 0;
            let dead_child_pid = unsafe { libc::waitpid(-1, &mut wait_status, libc::WNOHANG) };
            debug!("Reaping remaining processes in the process group. Waited [status={:?}; dead_child_pid={:?}]", wait_status, dead_child_pid);
            if dead_child_pid == -1 {
                break;
            } else if dead_child_pid == 0 {
                thread::sleep(time::Duration::from_millis(100));
                continue;
            }
        }
    }

    fn kill_process_group(&self) -> () {
        debug!("Killing the whole process group with SIGKILL");
        unsafe { libc::kill(0, unix::libc::SIGKILL) };
    }

    fn process_exit(self) -> ! {
        thread::sleep(time::Duration::from_millis(1000));
        debug!("Process exit. Bye!");
        process::exit(0)
    }

    fn handle_signal_received_sigchld(mut self) -> Box<Future<Item=Self, Error=()>> {
        debug!("Received SIGCHLD. Invoking waitpid");
        let mut wait_status = 0;
        let dead_child_pid = unsafe { libc::waitpid(-1, &mut wait_status, libc::WNOHANG) } as u32;

        debug!("Handling SIGCHLD. Waited [status={:?}; dead_child_pid={:?}]", wait_status, dead_child_pid);
        self.immediate_children.remove(&dead_child_pid);

        if self.immediate_children.len() == 0 {
            debug!("No immediate children left. Cleaning up and shutting down.");
            self.kill_process_group();
            self.reap_remaining_children();
            self.process_exit()
        } else {
            Box::new(future::ok(self))
        }
    }

    fn handle_signal_received_sigterm(mut self) -> Box<Future<Item=Self, Error=()>> {
        debug!("Received either SIGTERM or SIGINT. Initiating shutdown.");
        for child_pid in &self.immediate_children {
            unsafe { libc::kill(*child_pid as i32, unix::libc::SIGTERM) };
        }
        self.termination_timeout_tx.try_send(()).unwrap();
        Box::new(future::ok(self))
    }

    fn handle_signal_received(self, sig_num: i32) -> Box<Future<Item=Self, Error=()>> {
        debug!("Signal received [sig_num={:?}]", sig_num);
        match sig_num {
            unix::libc::SIGCHLD =>
                self.handle_signal_received_sigchld(),

            unix::libc::SIGTERM =>
                self.handle_signal_received_sigterm(),

            unix::libc::SIGINT =>
                self.handle_signal_received_sigterm(),

            _ =>
                Box::new(future::ok(self)),
        }
    }

    fn handle_termination_timeout(self) -> ! {
        warn!("Timed out waiting for child's graceful termination.");
        self.kill_process_group();
        self.reap_remaining_children();
        self.process_exit()
    }

    fn handle_io_error(self, io_err: Arc<io::Error>) -> ! {
        error!("IO-Error occurred [err={:?}]", io_err);
        self.kill_process_group();
        self.process_exit()
    }

    pub fn handle(self, event: Event) -> Box<Future<Item=Self, Error=()>> {
        match event {
            Event::ChildBorn(child_pid) =>
                self.handle_child_born(child_pid),

            Event::Signal(sig_num) =>
                self.handle_signal_received(sig_num),

            Event::TerminationTimeout =>
                self.handle_termination_timeout(),

            Event::IOError(io_err) =>
                self.handle_io_error(io_err),
        }
    }
}

pub fn new(termination_timeout_tx: mpsc::Sender<()>) -> Dispatcher {
    Dispatcher{
        termination_timeout_tx: termination_timeout_tx,
        immediate_children: HashSet::new(),
    }
}

