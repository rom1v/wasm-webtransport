use wasm_bindgen::prelude::*;
use web_sys::console;

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

fn document() -> web_sys::Document {
    let window = web_sys::window().expect("no global `window` exists");
    window.document().expect("should have a document on window")
}

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    // Use `web_sys`'s global `window` function to get a handle on the global
    // window object.
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let body = document.body().expect("document should have a body");

    // Manufacture the element we're gonna append
    let val = document.create_element("p")?;
    val.set_inner_html("Hello from Rust!");

    body.append_child(&val)?;

    let button = document.get_element_by_id("hello").expect("no 'hello' button");
    let button = button.dyn_into::<web_sys::HtmlElement>().unwrap();
    let on_click: Box<dyn FnMut(_)> = Box::new(move |_event: web_sys::InputEvent| {
        console::log_1(&JsValue::from_str("clicked"));
    });
    let on_click = Closure::wrap(on_click);
    button.add_event_listener_with_callback("click", on_click.as_ref().unchecked_ref())?;
    on_click.forget();

    console::log_1(&JsValue::from_str("ok"));

    Ok(())
}

#[wasm_bindgen]
pub fn hello() {
    console::log_1(&JsValue::from_str("Hello world!"));
}

#[wasm_bindgen]
pub fn wtconnect() -> Result<(), JsValue> {
    let document = document();
    let url = document.get_element_by_id("url").expect("No url element");
    let url = url.dyn_into::<web_sys::HtmlInputElement>().unwrap().value();
    console::log_1(&JsValue::from_str(&url));
    Ok(())
}
