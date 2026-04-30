use wasm_bindgen::JsValue;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Page {
    Liked,
    Search { q: String },
    Album { id: String },
    Artist { id: String },
    Event { id: String },
    Relation { artist_id: Option<String> },
    Settings,
}

pub(crate) fn read_current_page() -> Page {
    let Some(window) = web_sys::window() else {
        return Page::Liked;
    };
    let location = window.location();
    let path = location.pathname().unwrap_or_default();
    let search = location.search().unwrap_or_default();
    Page::from_location(&path, &search)
}

pub(crate) fn write_history(page: &Page, replace: bool) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(history) = window.history() else {
        return;
    };
    let url = page.to_path();
    if replace {
        let _ = history.replace_state_with_url(&JsValue::NULL, "", Some(&url));
    } else {
        let _ = history.push_state_with_url(&JsValue::NULL, "", Some(&url));
    }
}

impl Page {
    fn from_location(path: &str, search: &str) -> Self {
        let params = web_sys::UrlSearchParams::new_with_str(search).ok();
        let path = path.trim_end_matches('/');
        match path {
            "" | "/" | "/liked" => Page::Liked,
            "/search" => Page::Search {
                q: params
                    .as_ref()
                    .and_then(|params| params.get("q"))
                    .unwrap_or_default(),
            },
            "/relations" => Page::Relation {
                artist_id: params
                    .as_ref()
                    .and_then(|params| params.get("artist_id"))
                    .filter(|value| !value.is_empty()),
            },
            "/settings" => Page::Settings,
            _ if path.starts_with("/albums/") => Page::Album {
                id: decode_path_segment(path.trim_start_matches("/albums/")),
            },
            _ if path.starts_with("/artists/") => Page::Artist {
                id: decode_path_segment(path.trim_start_matches("/artists/")),
            },
            _ if path.starts_with("/events/") => Page::Event {
                id: decode_path_segment(path.trim_start_matches("/events/")),
            },
            _ => Page::Liked,
        }
    }

    pub(crate) fn to_path(&self) -> String {
        match self {
            Page::Liked => String::from("/liked"),
            Page::Search { q } if q.is_empty() => String::from("/search"),
            Page::Search { q } => format!("/search?q={}", urlencoding::encode(q)),
            Page::Album { id } => format!("/albums/{}", urlencoding::encode(id)),
            Page::Artist { id } => format!("/artists/{}", urlencoding::encode(id)),
            Page::Event { id } => format!("/events/{}", urlencoding::encode(id)),
            Page::Relation { artist_id: None } => String::from("/relations"),
            Page::Relation {
                artist_id: Some(id),
            } => format!("/relations?artist_id={}", urlencoding::encode(id)),
            Page::Settings => String::from("/settings"),
        }
    }
}

fn decode_path_segment(value: &str) -> String {
    urlencoding::decode(value)
        .map(|value| value.into_owned())
        .unwrap_or_else(|_| value.to_string())
}
