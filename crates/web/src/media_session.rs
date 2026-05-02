use easy_musiclib_macros::{js_function, js_get, js_set};
use easy_musiclib_shared::{Id, TrackSummary};
use std::rc::Rc;
use wasm_bindgen::{JsValue, prelude::Closure};

pub(crate) fn update_media_session(track: &TrackSummary, title_override: Option<&str>) {
    let Some(session) = browser_media_session() else {
        return;
    };
    let Some(metadata) = media_metadata(track, title_override) else {
        return;
    };
    set_js_property(&session, "metadata", &metadata);
}

pub(crate) fn update_media_position_state(position: f64, duration: f64, playback_rate: f64) {
    if !duration.is_finite() || duration <= 0.0 || !position.is_finite() {
        return;
    }
    let Some(session) = browser_media_session() else {
        return;
    };
    let Ok(set_position_state) = js_function!(&session, "setPositionState") else {
        return;
    };
    let state = js_sys::Object::new();
    set_js_property(
        state.as_ref(),
        "duration",
        &JsValue::from_f64(duration.max(0.0)),
    );
    set_js_property(
        state.as_ref(),
        "playbackRate",
        &JsValue::from_f64(if playback_rate.is_finite() {
            playback_rate
        } else {
            1.0
        }),
    );
    set_js_property(
        state.as_ref(),
        "position",
        &JsValue::from_f64(position.clamp(0.0, duration)),
    );
    let _ = set_position_state.call1(&session, state.as_ref());
}

pub(crate) fn install_media_seek_handlers(
    seek_to: impl Fn(f64) + 'static,
    seek_by: impl Fn(f64) + 'static,
) {
    let Some(session) = browser_media_session() else {
        return;
    };
    let Ok(set_action_handler) = js_function!(&session, "setActionHandler") else {
        return;
    };

    let seek_to = Rc::new(seek_to);
    let seek_by = Rc::new(seek_by);

    let seek_to_handler = Closure::<dyn FnMut(JsValue)>::wrap(Box::new({
        let seek_to = seek_to.clone();
        move |details| {
            if let Some(position) = numeric_detail(&details, "seekTime") {
                seek_to(position);
            }
        }
    }));
    let _ = set_action_handler.call2(
        &session,
        &JsValue::from_str("seekto"),
        seek_to_handler.as_ref(),
    );
    seek_to_handler.forget();

    let seek_forward_handler = Closure::<dyn FnMut(JsValue)>::wrap(Box::new({
        let seek_by = seek_by.clone();
        move |details| {
            seek_by(numeric_detail(&details, "seekOffset").unwrap_or(10.0));
        }
    }));
    let _ = set_action_handler.call2(
        &session,
        &JsValue::from_str("seekforward"),
        seek_forward_handler.as_ref(),
    );
    seek_forward_handler.forget();

    let seek_backward_handler = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |details| {
        seek_by(-numeric_detail(&details, "seekOffset").unwrap_or(10.0));
    }));
    let _ = set_action_handler.call2(
        &session,
        &JsValue::from_str("seekbackward"),
        seek_backward_handler.as_ref(),
    );
    seek_backward_handler.forget();
}

pub(crate) fn install_media_track_handlers(
    previous_track: impl Fn() + 'static,
    next_track: impl Fn() + 'static,
) {
    let Some(session) = browser_media_session() else {
        return;
    };
    let Ok(set_action_handler) = js_function!(&session, "setActionHandler") else {
        return;
    };

    let previous_track = Rc::new(previous_track);
    let next_track = Rc::new(next_track);

    let previous_handler = Closure::<dyn FnMut(JsValue)>::wrap(Box::new({
        let previous_track = previous_track.clone();
        move |_| {
            previous_track();
        }
    }));
    let _ = set_action_handler.call2(
        &session,
        &JsValue::from_str("previoustrack"),
        previous_handler.as_ref(),
    );
    previous_handler.forget();

    let next_handler = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_| {
        next_track();
    }));
    let _ = set_action_handler.call2(
        &session,
        &JsValue::from_str("nexttrack"),
        next_handler.as_ref(),
    );
    next_handler.forget();
}

fn numeric_detail(details: &JsValue, key: &str) -> Option<f64> {
    js_get!(details, key)
        .ok()
        .and_then(|value| value.as_f64())
        .filter(|value| value.is_finite())
}

fn browser_media_session() -> Option<JsValue> {
    let window = web_sys::window()?;
    let navigator = js_get!(window.as_ref(), "navigator").ok()?;
    let session = js_get!(&navigator, "mediaSession").ok()?;
    (!session.is_undefined() && !session.is_null()).then_some(session)
}

fn media_metadata(track: &TrackSummary, title_override: Option<&str>) -> Option<JsValue> {
    let window = web_sys::window()?;
    let constructor = js_function!(window.as_ref(), "MediaMetadata").ok()?;
    let init = js_sys::Object::new();
    set_js_property(
        init.as_ref(),
        "title",
        &JsValue::from_str(title_override.unwrap_or(&track.title)),
    );
    set_js_property(
        init.as_ref(),
        "artist",
        &JsValue::from_str(
            &track
                .artists
                .iter()
                .map(|artist| artist.name.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        ),
    );
    set_js_property(
        init.as_ref(),
        "album",
        &JsValue::from_str(
            track
                .album
                .as_ref()
                .map(|album| album.name.as_str())
                .unwrap_or(""),
        ),
    );
    if let Some(artwork_id) = track.artwork_id {
        set_js_property(init.as_ref(), "artwork", artwork_array(artwork_id).as_ref());
    }
    let args = js_sys::Array::new();
    args.push(init.as_ref());
    js_sys::Reflect::construct(&constructor, &args).ok()
}

fn artwork_array(artwork_id: Id) -> js_sys::Array {
    let artwork = js_sys::Array::new();
    for size in [96, 128, 192, 256, 384, 512] {
        let image = js_sys::Object::new();
        set_js_property(
            image.as_ref(),
            "src",
            &JsValue::from_str(&format!("/api/artwork/{artwork_id}?size={size}")),
        );
        set_js_property(
            image.as_ref(),
            "sizes",
            &JsValue::from_str(&format!("{size}x{size}")),
        );
        set_js_property(image.as_ref(), "type", &JsValue::from_str("image/jpeg"));
        artwork.push(image.as_ref());
    }
    artwork
}

fn set_js_property(target: &JsValue, key: &str, value: &JsValue) {
    let _ = js_set!(target, key, value);
}
pub(crate) fn js_error_text(value: JsValue) -> String {
    let name = js_string_property(&value, "name");
    let message = js_string_property(&value, "message");
    let text = value
        .as_string()
        .filter(|value| !value.is_empty())
        .or_else(|| js_to_string(&value));
    let detail = match (name.as_deref(), message.as_deref(), text.as_deref()) {
        (Some(name), Some(message), _) if !name.is_empty() && !message.is_empty() => {
            format!("{name}: {message}")
        }
        (Some(name), _, _) if !name.is_empty() => name.to_string(),
        (_, Some(message), _) if !message.is_empty() => message.to_string(),
        (_, _, Some(text)) if !text.is_empty() && text != "[object Object]" => text.to_string(),
        _ => String::new(),
    };

    if detail.is_empty() {
        String::from("Playback failed")
    } else {
        format!("Playback failed: {detail}")
    }
}

fn js_string_property(value: &JsValue, key: &str) -> Option<String> {
    js_get!(value, key)
        .ok()
        .and_then(|value| value.as_string())
        .filter(|value| !value.is_empty())
}

fn js_to_string(value: &JsValue) -> Option<String> {
    let function = js_function!(value, "toString").ok()?;
    function
        .call0(value)
        .ok()
        .and_then(|value| value.as_string())
        .filter(|value| !value.is_empty())
}
