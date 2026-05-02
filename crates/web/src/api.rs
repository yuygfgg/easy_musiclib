use crate::util::{PAGE_SIZE, pretty_json};
use easy_musiclib_shared::{ApiError, Id, ListResponse, ScanJobStatus};
use gloo_net::http::{Request, Response};
use leptos::prelude::*;
use serde::Serialize;
use serde::de::DeserializeOwned;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::Closure;

pub(crate) fn spawn_list_load<T>(
    url: String,
    target_page: i64,
    set_items: WriteSignal<Vec<T>>,
    set_page: WriteSignal<i64>,
    set_total: WriteSignal<i64>,
    set_status: WriteSignal<String>,
    label: &'static str,
) where
    T: DeserializeOwned + Send + Sync + 'static,
{
    wasm_bindgen_futures::spawn_local(async move {
        match api_get::<ListResponse<T>>(&url).await {
            Ok(data) => {
                let total = data.total.unwrap_or(data.items.len() as i64);
                set_items.set(data.items);
                set_page.set(target_page);
                set_total.set(total);
                set_status.set(format!("{total} {label}"));
            }
            Err(err) => set_status.set(err),
        }
    });
}

pub(crate) fn start_scan_poll(job_id: Id, set_scan_status: WriteSignal<String>) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let interval_id = std::rc::Rc::new(std::cell::Cell::new(0));
    let interval_for_callback = interval_id.clone();
    let closure = Closure::<dyn FnMut()>::wrap(Box::new(move || {
        let interval_for_callback = interval_for_callback.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match api_get::<ScanJobStatus>(&format!("/api/scan-jobs/{job_id}")).await {
                Ok(status) => {
                    let done = matches!(status.status.as_str(), "completed" | "failed");
                    set_scan_status.set(pretty_json(&status));
                    if done {
                        if let Some(window) = web_sys::window() {
                            window.clear_interval_with_handle(interval_for_callback.get());
                        }
                    }
                }
                Err(err) => set_scan_status.set(err),
            }
        });
    }));
    if let Ok(handle) = window.set_interval_with_callback_and_timeout_and_arguments_0(
        closure.as_ref().unchecked_ref(),
        1500,
    ) {
        interval_id.set(handle);
        closure.forget();
    }
}

pub(crate) async fn api_get<T>(url: &str) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let response = Request::get(url)
        .send()
        .await
        .map_err(|err| err.to_string())?;
    decode_response(response).await
}

pub(crate) async fn api_post_json<T, B>(url: &str, body: &B) -> Result<T, String>
where
    T: DeserializeOwned,
    B: Serialize + ?Sized,
{
    let response = Request::post(url)
        .json(body)
        .map_err(|err| err.to_string())?
        .send()
        .await
        .map_err(|err| err.to_string())?;
    decode_response(response).await
}

pub(crate) async fn api_patch_json<T, B>(url: &str, body: &B) -> Result<T, String>
where
    T: DeserializeOwned,
    B: Serialize + ?Sized,
{
    let response = Request::patch(url)
        .json(body)
        .map_err(|err| err.to_string())?
        .send()
        .await
        .map_err(|err| err.to_string())?;
    decode_response(response).await
}

async fn decode_response<T>(response: Response) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let ok = response.ok();
    let status = response.status_text();
    let text = response.text().await.map_err(|err| err.to_string())?;
    if ok {
        serde_json::from_str(&text).map_err(|err| format!("Decode failed: {err}"))
    } else if let Ok(error) = serde_json::from_str::<ApiError>(&text) {
        Err(error.message)
    } else if text.trim().is_empty() {
        Err(status)
    } else {
        Err(text)
    }
}

pub(crate) fn list_url(kind: &str, params: &[(&str, String)], page: i64) -> String {
    let mut query = vec![
        format!("limit={PAGE_SIZE}"),
        format!("offset={}", (page.max(1) - 1) * PAGE_SIZE),
    ];
    for (key, value) in params {
        if !value.is_empty() {
            query.push(format!("{key}={}", urlencoding::encode(value)));
        }
    }
    format!("/api/{kind}?{}", query.join("&"))
}
