macro_rules! clog {
    ($($e:expr),*) => {
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!($($e),*)));
    }
}
pub(crate) use clog;
