use js_sys;
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
    close_cbs: Option<CloseCallbacks>,
    web_transport: Option<web_sys::WebTransport>,
    datagram_writer: Option<web_sys::WritableStreamDefaultWriter>,
    stream_number: u32,
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
            close_cbs: None,
            web_transport: None,
            datagram_writer: None,
            stream_number: 1,
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

        let logger = self.logger.clone();
        let then = Closure::once(move |_| {
            logger.add_to_event_log(&"Connection closed normally.");
        });

        let logger = self.logger.clone();
        let catch = Closure::once(move |_| {
            logger.add_to_event_log(&"Connection closed abruptly.");
        });

        // Keep the closures alive
        self.close_cbs = Some(CloseCallbacks { then, catch });
        let cbs = self.close_cbs.as_ref().unwrap();
        let _ = web_transport.closed().then2(&cbs.then, &cbs.catch);

        self.web_transport = Some(web_transport.clone());

        let datagram_writer = web_transport
            .datagrams()
            .writable()
            .get_writer()
            .or_else(|err| {
                let msg = format!("Sending datagrams not supported: {:?}", err);
                self.logger.add_to_event_log_error(&msg);
                Err(JsValue::from(&msg))
            })?;
        self.datagram_writer = Some(datagram_writer);

        self.logger.add_to_event_log(&"Datagram writer ready.");

        self.read_datagrams(&web_transport).await?;
        self.accept_unidirectional_streams(&web_transport).await?;

        let send_button = self
            .document
            .get_element_by_id("send")
            .expect("No send button")
            .dyn_into::<web_sys::HtmlInputElement>()?;
        send_button.set_disabled(false);

        let connect_button = self
            .document
            .get_element_by_id("connect")
            .expect("No connect button")
            .dyn_into::<web_sys::HtmlInputElement>()?;
        connect_button.set_disabled(true);

        console::log_1(&JsValue::from_str(&url));

        Ok(())
    }

    pub fn send_data(&self) -> Result<(), JsValue> {
        Ok(())
    }

    async fn read_datagrams(&self, web_transport: &web_sys::WebTransport) -> Result<(), JsValue> {
        let datagram_reader = web_transport
            .datagrams()
            .readable()
            .get_reader()
            .dyn_into::<web_sys::ReadableStreamDefaultReader>()
            .or_else(|obj| {
                let msg = format!("Receiving datagrams not supported: {:?}", obj);
                self.logger.add_to_event_log_error(&msg);
                Err(JsValue::from(&msg))
            })?;

        self.logger.add_to_event_log(&"Datagram reader ready.");

        let decoder = web_sys::TextDecoder::new_with_label("utf-8").unwrap();
        loop {
            let obj = JsFuture::from(datagram_reader.read())
                .await
                .or_else(|err| {
                    let msg = format!("Error while reading datagrams: {:?}", err);
                    self.logger.add_to_event_log_error(&msg);
                    Err(JsValue::from(&msg))
                })?;
            let done = js_sys::Reflect::get(&obj, &JsValue::from("done"))?
                .as_bool()
                .unwrap_or(false);
            if done {
                self.logger.add_to_event_log(&"Done reading datagrams!");
                break;
            }

            let value = js_sys::Reflect::get(&obj, &JsValue::from("value"))?;
            assert!(!value.is_array());
            let value = value.dyn_into::<js_sys::Object>()?;
            let data = decoder.decode_with_buffer_source(&value)?;
            self.logger
                .add_to_event_log(&format!("Datagram received: {}", data));
        }

        Ok(())
    }

    async fn accept_unidirectional_streams(
        &mut self,
        web_transport: &web_sys::WebTransport,
    ) -> Result<(), JsValue> {
        let unistreams_reader = web_transport
            .incoming_unidirectional_streams()
            .get_reader()
            .dyn_into::<web_sys::ReadableStreamDefaultReader>()
            .or_else(|obj| {
                let msg = format!("Could not get unistream reader: {:?}", obj);
                self.logger.add_to_event_log_error(&msg);
                Err(JsValue::from(&msg))
            })?;

        loop {
            let obj = JsFuture::from(unistreams_reader.read())
                .await
                .or_else(|err| {
                    let msg = format!("Error while accepting streams: {:?}", err);
                    self.logger.add_to_event_log_error(&msg);
                    Err(JsValue::from(&msg))
                })?;
            let done = js_sys::Reflect::get(&obj, &JsValue::from("done"))?
                .as_bool()
                .unwrap_or(false);
            if done {
                self.logger
                    .add_to_event_log(&"Done accepting unidirectional streams!");
                break;
            }

            let stream = js_sys::Reflect::get(&obj, &JsValue::from("value"))?;
            let stream = stream
                .dyn_into::<web_sys::WebTransportReceiveStream>()
                .unwrap();
            let number = self.stream_number;
            self.stream_number += 1;
            self.logger
                .add_to_event_log(&format!("New incoming unidirectional stream #{number}"));
            self.read_from_incoming_stream(&stream, number).await?;
        }

        Ok(())
    }

    async fn read_from_incoming_stream(
        &self,
        stream: &web_sys::WebTransportReceiveStream,
        number: u32,
    ) -> Result<(), JsValue> {
        let stream_reader = stream
            .get_reader()
            .dyn_into::<web_sys::ReadableStreamDefaultReader>()
            .or_else(|obj| {
                let msg = format!("Could not get stream reader: {:?}", obj);
                self.logger.add_to_event_log_error(&msg);
                Err(JsValue::from(&msg))
            })?;
        let decoder = web_sys::TextDecoder::new_with_label("utf-8").unwrap();

        loop {
            let obj = JsFuture::from(stream_reader.read()).await.or_else(|err| {
                let msg = format!("Error while reading stream #{number}: {:?}", err);
                self.logger.add_to_event_log_error(&msg);
                Err(JsValue::from(&msg))
            })?;
            let done = js_sys::Reflect::get(&obj, &JsValue::from("done"))?
                .as_bool()
                .unwrap_or(false);
            if done {
                self.logger
                    .add_to_event_log(&format!("Stream #{number} closed"));
                break;
            }

            let value = js_sys::Reflect::get(&obj, &JsValue::from("value"))?;
            assert!(!value.is_array());
            let value = value.dyn_into::<js_sys::Object>()?;
            let data = decoder.decode_with_buffer_source(&value)?;
            self.logger
                .add_to_event_log(&format!("Datagram received: {}", data));
        }

        Ok(())
    }
}

struct CloseCallbacks {
    then: Closure<dyn FnMut(JsValue)>,
    catch: Closure<dyn FnMut(JsValue)>,
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
