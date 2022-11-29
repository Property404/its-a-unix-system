use crate::{
    streams::{
        input_stream::InputMode, Backend, InputStream, OutputStream, TerminalReader, TerminalWriter,
    },
    utils,
};
use anyhow::{anyhow, Result};
use futures::{
    channel::{
        mpsc::{self, UnboundedReceiver, UnboundedSender},
        oneshot,
    },
    stream::{FusedStream, Stream},
};
use std::{
    pin::Pin,
    task::{Context, Poll},
};
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{self, KeyboardEvent};

pub type InitializationTuple = (
    InputStream,
    OutputStream,
    Backend<KeyboardTerminalReader, HtmlTerminalWriter>,
    UnboundedSender<oneshot::Sender<()>>,
);

pub fn standard() -> Result<InitializationTuple> {
    let (signal_registrar_tx, signal_registrar_rx) = mpsc::unbounded();
    let writer = HtmlTerminalWriter::default();
    let (output_stream, output_bkend) = OutputStream::from_writer(writer);
    let reader = KeyboardTerminalReader::new(signal_registrar_rx)?;
    let (input_stream, input_bkend) = InputStream::from_reader(reader);

    let backend = Backend {
        input_bkend,
        output_bkend,
    };

    Ok((input_stream, output_stream, backend, signal_registrar_tx))
}

pub struct KeyboardTerminalReader {
    callback: Closure<dyn FnMut(KeyboardEvent)>,
    mode_tx: UnboundedSender<InputMode>,
    stream: UnboundedReceiver<Vec<u8>>,
}

impl TerminalReader for KeyboardTerminalReader {
    fn set_mode(&mut self, mode: InputMode) -> Result<()> {
        self.mode_tx.start_send(mode)?;
        Ok(())
    }
}

impl Stream for KeyboardTerminalReader {
    type Item = Vec<u8>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.stream).poll_next(cx)
    }
}

impl FusedStream for KeyboardTerminalReader {
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated()
    }
}

fn unix_term_escape(src: &str) -> String {
    let mut string = String::with_capacity(src.len());
    for c in src.chars() {
        if c as u8 <= 0x1F && c != '\t' && c != '\n' {
            string.push('^');
            string.push((c as u8 + 0x40) as char);
        } else {
            string.push(c);
        }
    }
    string
}

// This is a "Sit Still and Look Pretty" struct.
// Just existing should be enough for it to...do things.
impl KeyboardTerminalReader {
    fn new(
        mut signal_registrar: UnboundedReceiver<oneshot::Sender<()>>,
    ) -> Result<KeyboardTerminalReader> {
        let document = utils::get_document()?;
        let (sender, receiver) = mpsc::unbounded();
        let (mode_tx, mut mode_rx) = mpsc::unbounded();
        let mut cbuffer = Vec::<u8>::new();

        let mut mode = InputMode::Line;

        let callback = Closure::new(move |e: KeyboardEvent| {
            let key = e.key();

            match mode_rx.try_next() {
                Ok(Some(new_mode)) => {
                    mode = new_mode;
                }
                _ => {}
            };

            fn echo(mode: InputMode, content: &str, buffer: &mut Vec<u8>) {
                buffer.extend(content.as_bytes());
                if mode == InputMode::Line {
                    let content = unix_term_escape(content);
                    utils::js_term_write(&content);
                }
            }

            if e.ctrl_key() && key == "c" {
                e.prevent_default();
                while let Ok(Some(channel)) = signal_registrar.try_next() {
                    // We don't care if the channel is closed
                    // It just means the process is probably dead
                    let _ = channel.send(());
                }
                utils::js_term_write("^C");
                cbuffer.clear();
                return;
            }

            if key.len() == 1 {
                echo(mode, &key, &mut cbuffer);
                if "'/?".contains(&key) {
                    e.prevent_default();
                }
            } else if key == "Tab" {
                e.prevent_default();
                echo(mode, "\t", &mut cbuffer);
            } else if key == "ArrowLeft" {
                echo(mode, "\x1b[D", &mut cbuffer);
            } else if key == "Enter" {
                echo(mode, "\n", &mut cbuffer);
                if mode == InputMode::Line {
                    sender
                        .unbounded_send(cbuffer.clone())
                        .expect("Send failed :(");

                    cbuffer.clear();
                }
            } else if key == "Backspace" {
                if mode == InputMode::Line && !cbuffer.is_empty() {
                    utils::js_term_backspace();
                    cbuffer.pop();
                }

                if mode == InputMode::Char {
                    cbuffer.push(b'\x08')
                }
            }

            if mode == InputMode::Char && !cbuffer.is_empty() {
                sender
                    .unbounded_send(cbuffer.clone())
                    .expect("Send failed :(");
                cbuffer.clear();
            }
        });
        document
            .add_event_listener_with_callback("keydown", callback.as_ref().as_ref().unchecked_ref())
            .map_err(|_| anyhow!("Failed to set event handler"))?;

        Ok(Self {
            callback,
            stream: receiver,
            mode_tx,
        })
    }
}

impl Drop for KeyboardTerminalReader {
    fn drop(&mut self) {
        let document = utils::get_document().expect("Failed to get document");
        let _ = document.remove_event_listener_with_callback(
            "keydown",
            self.callback.as_ref().as_ref().unchecked_ref(),
        );
    }
}

#[derive(Default, Clone)]
pub struct HtmlTerminalWriter {}

impl TerminalWriter for HtmlTerminalWriter {
    fn send(&mut self, content: &str) -> Result<()> {
        utils::js_term_write(content);
        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }

    fn to_terminal(&self) -> bool {
        true
    }
}
