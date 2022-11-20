use crate::streams::{Backend, InputStream, OutputStream, TerminalReader, TerminalWriter};
use crate::utils;
use anyhow::{anyhow, bail, Result};
use futures::{
    channel::mpsc::{self, UnboundedReceiver},
    stream::{FusedStream, Stream},
};
use std::{
    io::Write,
    pin::Pin,
    task::{Context, Poll},
};
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{self, Element, KeyboardEvent};

pub fn standard() -> Result<(
    InputStream,
    OutputStream,
    Backend<KeyboardTerminalReader, HtmlTerminalWriter>,
)> {
    let writer = HtmlTerminalWriter::new()?;
    let (output_stream, output_bkend) = OutputStream::from_writer(writer);
    let reader = KeyboardTerminalReader::new(output_stream.clone())?;
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
    fn new(mut out: OutputStream) -> Result<KeyboardTerminalReader> {
        let document = utils::get_document()?;
        let (sender, receiver) = mpsc::unbounded();
        let mut cbuffer = Vec::<u8>::new();

        let callback = Closure::new(move |e: KeyboardEvent| {
            let key = e.key();
            if key.len() == 1 {
                if "'/?".contains(&key) {
                    e.prevent_default();
                }
                let key: Vec<u8> = key.as_bytes().into();
                out.write_all(&key).expect("Failed to write");
                out.flush().expect("Failed to flush");
                cbuffer.extend(&key);
            } else if key == "Enter" {
                out.write_all(b"\n").expect("Failed to write");
                out.flush().expect("Failed to flush");
                cbuffer.push(b'\n');

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

pub struct HtmlTerminalWriter {
    root: Element,
}

impl HtmlTerminalWriter {
    fn new() -> Result<Self> {
        let Some(root) = utils::get_document()?.get_element_by_id("terminal") else {
            bail!("Could not find terminal element");
        };
        Ok(Self { root })
    }
}

impl TerminalWriter for HtmlTerminalWriter {
    fn send(&mut self, content: &str) -> Result<()> {
        let content = self.root.text_content().unwrap_or_default() + content;
        self.root.set_text_content(Some(&content));
        Ok(())
    }
}
