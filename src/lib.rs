use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::console;

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    console::log_1(&JsValue::from_str("ok"));

    Ok(())
}

#[wasm_bindgen]
pub struct WasmCtx {
    window: web_sys::Window,
    document: web_sys::Document,
    logger: Logger,
    web_transport: Option<web_sys::WebTransport>,
}

#[wasm_bindgen]
impl WasmCtx {
    pub fn new() -> Self {
        let window = web_sys::window().expect("no global `window` exists");
        let document = window.document().expect("should have a document on window");
        let logger = Logger::new(document.clone());
        Self {
            window,
            document,
            logger,
            web_transport: None,
        }
    }

    pub async fn connect(&mut self) -> Result<(), JsValue> {
        let url = self
            .document
            .get_element_by_id("url")
            .expect("No url element");
        let url = url.dyn_into::<web_sys::HtmlInputElement>().unwrap().value();

        let web_transport = web_sys::WebTransport::new(&url).or_else(|err| {
            let msg = format!("Failed to create connection object. {:?}", err);
            self.logger.add_to_event_log_error(&msg);
            Err(JsValue::from(&msg))
        })?;

        self.logger.add_to_event_log(&"Initiating connection...");

        JsFuture::from(web_transport.ready()).await.or_else(|err| {
            let msg = format!("Connection failed. {:?}", err);
            self.logger.add_to_event_log_error(&msg);
            Err(JsValue::from(&msg))
        })?;

        self.logger.add_to_event_log(&"Connection ready.");

        self.web_transport = Some(web_transport);

        console::log_1(&JsValue::from_str(&url));

        Ok(())
    }

    pub fn send_data(&self) -> Result<(), JsValue> {
        Ok(())
    }
}

#[derive(PartialEq, Eq, Copy, Clone)]
enum Severity {
    INFO,
    ERROR,
}

#[derive(Clone)]
struct Logger {
    document: web_sys::Document,
}

impl Logger {
    fn new(document: web_sys::Document) -> Self {
        Self { document }
    }

    fn add_to_event_log_with_severity(&self, text: &str, severity: Severity) {
        let log = self
            .document
            .get_element_by_id("event-log")
            .expect("no event-log");
        let most_recent_entry = log.last_element_child();
        let entry = self
            .document
            .create_element("li")
            .expect("Cannot create 'li'");
        entry.set_inner_html(text);
        let class_name = if severity == Severity::ERROR {
            "log-error"
        } else {
            "log-info"
        };
        entry.set_class_name(class_name);
        log.append_child(&entry).expect("Could not append child");

        // If the most recent entry in the log was visible, scroll the log to the
        // newly added element.
        if let Some(most_recent_entry) = most_recent_entry {
            if most_recent_entry.get_bounding_client_rect().top()
                < log.get_bounding_client_rect().bottom()
            {
                entry.scroll_into_view();
            }
        }
    }

    fn add_to_event_log(&self, text: &str) {
        self.add_to_event_log_with_severity(text, Severity::INFO);
    }

    fn add_to_event_log_error(&self, text: &str) {
        self.add_to_event_log_with_severity(text, Severity::ERROR);
    }
}
