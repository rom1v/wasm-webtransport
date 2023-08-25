use crate::log::clog;
use bytes::Bytes;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{spawn_local, JsFuture};

mod log;

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    clog!("ok");

    Ok(())
}

#[wasm_bindgen]
pub struct WasmCtx {
    document: web_sys::Document,
    logger: Logger,
    conn: Option<kyproto::Connection>,
    stream_number: Rc<RefCell<u32>>,
}

#[wasm_bindgen]
impl WasmCtx {
    pub fn new() -> Self {
        let window = web_sys::window().expect("no global `window` exists");
        let document = window.document().expect("should have a document on window");
        let logger = Logger::new(document.clone());
        Self {
            document,
            logger,
            conn: None,
            stream_number: Rc::new(RefCell::new(1)),
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

        self.logger.add_to_event_log("Initiating connection...");

        JsFuture::from(web_transport.ready()).await.or_else(|err| {
            let msg = format!("Connection failed. {:?}", err);
            self.logger.add_to_event_log_error(&msg);
            Err(JsValue::from(&msg))
        })?;

        self.logger.add_to_event_log("Connection ready.");

        let conn = kyproto::Connection::from(web_transport);

        let conn2 = conn.clone();
        let logger = self.logger.clone();
        spawn_local(async move {
            match conn2.closed().await {
                Ok(_) => logger.add_to_event_log("Connection closed normally."),
                Err(err) => {
                    logger.add_to_event_log(&format!("Connection closed abruptly: {err:?}"))
                }
            }
        });

        self.conn = Some(conn.clone());

        let logger = self.logger.clone();
        let conn2 = conn.clone();
        spawn_local(async move {
            let result = Self::read_datagrams(&logger, &conn2).await;
            if let Err(err) = result {
                clog!("Error while reading datagrams: {err:?}");
            }
        });

        let logger = self.logger.clone();
        let stream_number = self.stream_number.clone();
        spawn_local(async move {
            let result = Self::accept_unidirectional_streams(&logger, stream_number, &conn)
                .await;
            if let Err(err) = result {
                clog!("Error while accepting uni streams: {err:?}");
            }
        });

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

        Ok(())
    }

    pub async fn send_data(&self) -> Result<(), JsValue> {
        let value = self
            .document
            .get_element_by_id("data")
            .expect("No data element")
            .dyn_into::<web_sys::HtmlTextAreaElement>()
            .unwrap()
            .value();

        let conn = self.conn.as_ref().expect("No connection");

        let selected = self.get_selected_radio_value().expect("No radio selected");
        match selected.as_str() {
            "datagram" => {
                let bytes = Bytes::from(value.as_bytes().to_vec());
                conn.send_datagram(bytes).await?;
                self.logger
                    .add_to_event_log(&format!("Sent datagram: {value}"));
            }
            "unidi" => {
                let mut uni = conn.open_uni().await?;
                uni.write_all(value.as_bytes()).await?;
                uni.close().await?;
                self.logger
                    .add_to_event_log(&format!("Sent a unidirectional stream with data: {value}"));
            }
            "bidi" => {
                let (mut send, recv) = conn.open_bi().await?;

                let number = {
                    let mut stream_number = self.stream_number.borrow_mut();
                    let number = *stream_number;
                    *stream_number += 1;
                    number
                };

                send.write_all(value.as_bytes()).await?;
                send.close().await?;
                self.logger.add_to_event_log(&format!(
                    "Opened bidirectional stream #{number} with data: {value}"
                ));

                let logger = self.logger.clone();
                spawn_local(async move {
                    let result = Self::read_from_incoming_stream(&logger, recv, number)
                        .await;
                    if let Err(err) = result {
                        clog!("Error while reading stream {number}: {err:?}");
                    }
                });
            }
            _ => {
                Err(JsValue::from(&format!("Unexpected selection: {selected}")))?;
            }
        }

        Ok(())
    }

    fn get_selected_radio_value(&self) -> Option<String> {
        self.document
            .query_selector("#sending input[name=\"sendtype\"]:checked")
            .expect("No selection")
            .map(|element| {
                element
                    .dyn_into::<web_sys::HtmlInputElement>()
                    .unwrap()
                    .value()
            })
    }

    async fn read_datagrams(logger: &Logger, conn: &kyproto::Connection) -> Result<(), JsValue> {
        loop {
            let datagram = conn.read_datagram().await?;
            let data = String::from_utf8_lossy(&datagram);
            logger.add_to_event_log(&format!("Datagram received: {}", data));
        }
    }

    async fn accept_unidirectional_streams(
        logger: &Logger,
        stream_number: Rc<RefCell<u32>>,
        conn: &kyproto::Connection,
    ) -> Result<(), JsValue> {
        loop {
            let recv = conn.accept_uni().await?;
            let number = {
                let mut stream_number = stream_number.borrow_mut();
                let number = *stream_number;
                *stream_number += 1;
                number
            };
            logger.add_to_event_log(&format!("New incoming unidirectional stream #{number}"));
            let logger = logger.clone();
            spawn_local(async move {
                let result = Self::read_from_incoming_stream(&logger, recv, number)
                    .await;
                if let Err(err) = result {
                    clog!("Error while reading stream {number}: {err:?}");
                }
            });
        }
    }

    async fn read_from_incoming_stream(
        logger: &Logger,
        mut recv: kyproto::RecvStream,
        number: u32,
    ) -> Result<(), JsValue> {
        // We don't mind any additional copy
        let mut buf = vec![0; 1024];
        let mut vec = vec![];
        loop {
            let r = recv.read(&mut buf).await?;
            if let Some(r) = r {
                vec.extend_from_slice(&buf[..r]);
            } else {
                break;
            }
        }

        logger.add_to_event_log(&format!("Stream #{number} closed"));
        let data = String::from_utf8_lossy(&vec);
        logger.add_to_event_log(&format!("Data received: {}", data));

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
