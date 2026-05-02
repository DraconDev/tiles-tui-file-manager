use signal_hook::consts::signal::SIGWINCH;
use signal_hook::iterator::Signals;
use std::io::Read;
use std::os::fd::AsFd;
use std::thread;

use super::event::Event;
use super::parser::Parser;
use crate::backend::tty;

pub struct InputReader;

impl InputReader {
    pub fn spawn<F>(mut callback: F) -> thread::JoinHandle<()>
    where
        F: FnMut(Event) + Send + 'static,
    {
        thread::spawn(move || {
            let mut parser = Parser::new();
            let mut stdin = std::io::stdin();
            let mut buffer = [0; 1024];

            let mut signals = Signals::new([SIGWINCH]).expect("Failed to register signals");

            loop {
                // 1. Check Signals (Non-blocking)
                for signal in signals.pending() {
                    if signal == SIGWINCH {
                        if let Ok((w, h)) = tty::get_window_size(stdin.as_fd()) {
                            callback(Event::Resize(w, h));
                        }
                    }
                }

                // 2. Poll with 20ms timeout
                // Need to borrow stdin's FD
                let stdin_fd = stdin.as_fd();
                let polled = tty::poll_input(stdin_fd, 20);

                match polled {
                    Ok(true) => {
                        match stdin.read(&mut buffer) {
                            Ok(0) => break, // EOF
                            Ok(n) => {
                                for item in buffer.iter().take(n) {
                                    if let Some(event) = parser.advance(*item) {
                                        callback(event);
                                    }
                                }
                            }
                            Err(_) => break,
                        }
                    }
                    Ok(false) => {
                        // Timeout - check for incomplete sequences (like Esc)
                        if let Some(evt) = parser.check_timeout() {
                            callback(evt);
                        }
                    }
                    Err(_) => break,
                }
            }
        })
    }
}
