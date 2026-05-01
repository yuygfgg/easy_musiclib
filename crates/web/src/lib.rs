mod api;
mod app;
mod lyrics;
mod media_session;
mod pages;
mod player;
mod relation_layout;
mod route;
mod ui;
mod util;

#[wasm_bindgen::prelude::wasm_bindgen(start)]
#[cfg(target_arch = "wasm32")]
pub fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(app::App);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn main() {
    let _ = app::App;
}
