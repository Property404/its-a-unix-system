use anyhow::{bail, Result};
use wasm_bindgen::prelude::*;
use web_sys::{self, Document};

#[allow(unused)]
pub fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// Fetch DOM document object.
pub fn get_document() -> Result<Document> {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        bail!("Could not get root html document");
    };
    Ok(document)
}

#[wasm_bindgen]
extern "C" {
    pub fn js_term_write(s: &str);
    pub fn js_term_backspace();
    pub fn js_term_get_screen_height() -> usize;
}

#[allow(unused)]
pub fn debug<S: Into<String>>(s: S) {
    js_term_write(s.into().as_str());
}
