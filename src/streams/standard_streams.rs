use crate::{
    streams::{Backend, InputStream, OutputStream, TerminalReader, TerminalWriter},
    utils,
};
use anyhow::{anyhow, Result};
use futures::{
    channel::mpsc::{self, UnboundedReceiver},
    stream::{FusedStream, Stream},
};
use std::{
    pin::Pin,
    task::{Context, Poll},
};
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{self, KeyboardEvent};

pub fn standard() -> Result<(
    InputStream,
    OutputStream,
    Backend<KeyboardTerminalReader, HtmlTerminalWriter>,
)> {
    let writer = HtmlTerminalWriter::default();
    let (output_stream, output_bkend) = OutputStream::from_writer(writer);
    let reader = KeyboardTerminalReader::new()?;
    let (input_stream, input_bkend) = InputStream::from_reader(reader);

    let backend = Backend {
        input_bkend,
        output_bkend,
    };

    Ok((input_stream, output_stream, backend))
}

pub struct KeyboardTerminalReader {
    callback: Closure<dyn FnMut(KeyboardEvent)>,
    stream: UnboundedReceiver<Vec<u8>>,
}

impl TerminalReader for KeyboardTerminalReader {}

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

// This is a "Sit Still and Look Pretty" struct.
// Just existing should be enough for it to...do things.
impl KeyboardTerminalReader {
    fn new() -> Result<KeyboardTerminalReader> {
        let document = utils::get_document()?;
        let (sender, receiver) = mpsc::unbounded();
        let mut cbuffer = Vec::<u8>::new();

        let callback = Closure::new(move |e: KeyboardEvent| {
            let key = e.key();
            if key.len() == 1 {
                utils::js_term_write(&key);
                cbuffer.extend(key.as_bytes());
                if "'/?".contains(&key) {
                    e.prevent_default();
                }
            } else if key == "Enter" {
                utils::js_term_write("\n");
                cbuffer.push(b'\n');

                sender
                    .unbounded_send(cbuffer.clone())
                    .expect("Send failed :(");

                cbuffer.clear();
            } else if key == "Backspace" && !cbuffer.is_empty() {
                utils::js_term_backspace();
                cbuffer.pop();
            }
        });
        document
            .add_event_listener_with_callback("keydown", callback.as_ref().as_ref().unchecked_ref())
            .map_err(|_| anyhow!("Failed to set event handler"))?;

        Ok(Self {
            callback,
            stream: receiver,
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
}
