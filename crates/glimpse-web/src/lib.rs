// glimpse-web/src/lib.rs
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{console, CloseEvent, ErrorEvent, MessageEvent, WebSocket};

#[derive(Serialize, Deserialize)]
pub struct Metric {
    url: String,
    value: f64,
    timestamp: u64,
}

#[derive(Serialize, Deserialize)]
pub struct MetricsData {
    metrics: Vec<Metric>,
}

#[wasm_bindgen]
pub struct GlimpseApp {
    data: MetricsData,
    ws: Option<WebSocket>,
    callback: Option<js_sys::Function>,
}

#[wasm_bindgen]
impl GlimpseApp {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        console::log_1(&"Initializing GlimpseApp".into());
        Self {
            data: MetricsData {
                metrics: Vec::new(),
            },
            ws: None,
            callback: None,
        }
    }

    pub fn connect(&mut self, url: &str, callback: js_sys::Function) -> Result<(), JsValue> {
        let ws = WebSocket::new(url)?;

        // Set up message handler
        let onmessage_callback = Closure::wrap(Box::new(move |e: MessageEvent| {
            if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                let txt_string: String = txt.into();
                callback
                    .call1(&JsValue::NULL, &JsValue::from_str(&txt_string))
                    .expect("callback failed");
            }
        }) as Box<dyn FnMut(MessageEvent)>);

        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget();

        // Set up error handler
        let onerror_callback = Closure::wrap(Box::new(move |e: ErrorEvent| {
            console::error_1(&format!("WebSocket error: {:?}", e).into());
        }) as Box<dyn FnMut(ErrorEvent)>);

        ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        onerror_callback.forget();

        // Set up close handler
        let onclose_callback = Closure::wrap(Box::new(move |e: CloseEvent| {
            console::log_1(&format!("WebSocket closed: {:?}", e).into());
        }) as Box<dyn FnMut(CloseEvent)>);

        ws.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
        onclose_callback.forget();

        self.ws = Some(ws);
        self.callback = Some(callback);
        Ok(())
    }

    pub fn disconnect(&mut self) {
        if let Some(ws) = self.ws.take() {
            let _ = ws.close();
        }
        self.callback = None;
    }

    pub fn update_metrics(&mut self, json: String) -> Result<(), JsValue> {
        match serde_json::from_str(&json) {
            Ok(metrics) => {
                self.data = metrics;
                Ok(())
            }
            Err(e) => {
                console::error_1(&format!("Failed to parse metrics: {}", e).into());
                Err(JsValue::from_str("Failed to parse metrics"))
            }
        }
    }

    pub fn get_metrics_json(&self) -> String {
        serde_json::to_string(&self.data).unwrap_or_else(|_| "[]".to_string())
    }
}

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    console::log_1(&"WASM module initialized".into());
    Ok(())
}
