use easy_musiclib_shared::{Id, TrackSummary};
use wasm_bindgen::{JsCast, JsValue};

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
    let Ok(set_position_state) =
        js_sys::Reflect::get(&session, &JsValue::from_str("setPositionState"))
            .and_then(|value| value.dyn_into::<js_sys::Function>())
    else {
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

fn browser_media_session() -> Option<JsValue> {
    let window = web_sys::window()?;
    let navigator = js_sys::Reflect::get(window.as_ref(), &JsValue::from_str("navigator")).ok()?;
    let session = js_sys::Reflect::get(&navigator, &JsValue::from_str("mediaSession")).ok()?;
    (!session.is_undefined() && !session.is_null()).then_some(session)
}

fn media_metadata(track: &TrackSummary, title_override: Option<&str>) -> Option<JsValue> {
    let window = web_sys::window()?;
    let constructor = js_sys::Reflect::get(window.as_ref(), &JsValue::from_str("MediaMetadata"))
        .ok()?
        .dyn_into::<js_sys::Function>()
        .ok()?;
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
    let _ = js_sys::Reflect::set(target, &JsValue::from_str(key), value);
}
pub(crate) fn js_error_text(value: JsValue) -> String {
    value
        .as_string()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| String::from("Playback failed"))
}
