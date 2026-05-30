use bityzba::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = jbotciComputeHandle)]
#[requires(true)]
#[ensures(true)]
pub fn jbotci_compute_handle(request_json: &str) -> Result<String, JsValue> {
    compute_handle(request_json).map_err(|error| JsValue::from_str(&error))
}

#[requires(!request_json.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|json| !json.is_empty()) || ret.is_err())]
fn compute_handle(request_json: &str) -> Result<String, String> {
    jbotci_web_core::run_web_compute_request_json(request_json).map_err(|error| error.to_string())
}
