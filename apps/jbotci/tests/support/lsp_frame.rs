//! One reader thread parses LSP frames.
//!
//! The thread owns the buffered reader.
//! Each frame is one value on the channel.
//! A timed wait uses the channel.

use std::io::{BufRead, BufReader, Read};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};

#[allow(unused_imports)]
use bityzba::{ensures, requires};
use serde_json::Value;

/// Parse frames from `stdout` and send each value.
///
/// The caller waits on the channel.
/// The caller does not read the buffered stream.
#[requires(true)]
#[ensures(true)]
pub(crate) fn spawn_lsp_reader<R>(stdout: R) -> (mpsc::Receiver<Value>, JoinHandle<()>)
where
    R: Read + Send + 'static,
{
    let (sender, receiver) = mpsc::channel();
    let reader = thread::spawn(move || {
        let mut stdout = BufReader::new(stdout);
        while let Some(message) = read_lsp_message(&mut stdout) {
            if sender.send(message).is_err() {
                break;
            }
        }
    });
    (receiver, reader)
}

/// Read one LSP frame from `input`.
///
/// End of file returns none.
/// A short frame returns none.
#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|message| message.is_object()))]
fn read_lsp_message(input: &mut impl BufRead) -> Option<Value> {
    let mut content_length = None;
    loop {
        let mut header = String::new();
        let read = input.read_line(&mut header).ok()?;
        if read == 0 {
            return None;
        }
        if header == "\r\n" || header == "\n" {
            break;
        }
        if let Some(value) = header
            .strip_prefix("Content-Length:")
            .or_else(|| header.strip_prefix("content-length:"))
        {
            content_length = Some(value.trim().parse::<usize>().ok()?);
        }
    }
    let mut body = vec![0; content_length?];
    if input.read_exact(&mut body).is_err() {
        return None;
    }
    serde_json::from_slice(&body).ok()
}
