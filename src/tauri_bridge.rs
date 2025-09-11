use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke_raw(cmd: &str, args: JsValue) -> JsValue;
}

pub async fn invoke_cmd(cmd: &str, args: &serde_json::Value) -> JsValue {
    let s = serde_json::to_string(args).unwrap_or_else(|_| String::from("{}"));
    let js = JsValue::from_str(&s);
    invoke_raw(cmd, js).await
}

pub async fn invoke_empty(cmd: &str) -> JsValue {
    invoke_raw(cmd, JsValue::from_str("{}" )).await
}

