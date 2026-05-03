use crate::api::{api_post_json, spawn_auth_status_load};
use crate::pages::{
    AlbumPage, ArtistPage, EventPage, LikedPage, RelationPage, SearchPage, SettingsPage,
};
use crate::player::Player;
use crate::route::{Page, read_current_page, write_history};
use crate::util::nav_class;
use easy_musiclib_macros::{match_any_view, spawn_result};
use easy_musiclib_shared::{AuthStatusResponse, LoginRequest, TrackSummary};
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::Closure;

#[derive(Clone, Debug)]
pub(crate) struct PlayRequest {
    pub(crate) track: TrackSummary,
}

#[derive(Clone, Copy)]
pub(crate) struct AppContext {
    pub(crate) navigate: Callback<Page>,
    pub(crate) play_request: ReadSignal<Option<PlayRequest>>,
    pub(crate) set_play_request: WriteSignal<Option<PlayRequest>>,
    pub(crate) current_track: ReadSignal<Option<TrackSummary>>,
    pub(crate) set_current_track: WriteSignal<Option<TrackSummary>>,
    pub(crate) playlist: ReadSignal<Vec<TrackSummary>>,
    pub(crate) set_playlist: WriteSignal<Vec<TrackSummary>>,
    pub(crate) playlist_index: ReadSignal<i64>,
    pub(crate) set_playlist_index: WriteSignal<i64>,
    pub(crate) set_status: WriteSignal<String>,
    pub(crate) track_update: ReadSignal<Option<TrackSummary>>,
    pub(crate) set_track_update: WriteSignal<Option<TrackSummary>>,
}

#[component]
pub(crate) fn App() -> impl IntoView {
    let (page, set_page) = signal(read_current_page());
    let (shell_query, set_shell_query) = signal(String::new());
    let (play_request, set_play_request) = signal::<Option<PlayRequest>>(None);
    let (current_track, set_current_track) = signal::<Option<TrackSummary>>(None);
    let (playlist, set_playlist) = signal(Vec::<TrackSummary>::new());
    let (playlist_index, set_playlist_index) = signal(-1_i64);
    let (status, set_status) = signal(String::from("Ready"));
    let (track_update, set_track_update) = signal::<Option<TrackSummary>>(None);
    let (auth_status, set_auth_status) = signal::<Option<AuthStatusResponse>>(None);
    let (login_username, set_login_username) = signal(String::new());
    let (login_password, set_login_password) = signal(String::new());

    let navigate = Callback::new(move |target: Page| {
        write_history(&target, false);
        set_page.set(target);
    });

    provide_context(AppContext {
        navigate,
        play_request,
        set_play_request,
        current_track,
        set_current_track,
        playlist,
        set_playlist,
        playlist_index,
        set_playlist_index,
        set_status,
        track_update,
        set_track_update,
    });

    Effect::new(move |_| {
        if let Some(updated) = track_update.get() {
            set_current_track.update(|track| {
                if track.as_ref().map(|item| item.id) == Some(updated.id) {
                    *track = Some(updated.clone());
                }
            });
            set_playlist.update(|items| {
                for item in items {
                    if item.id == updated.id {
                        *item = updated.clone();
                    }
                }
            });
        }
    });

    Effect::new(move |_| {
        if let Page::Search { q } = page.get() {
            set_shell_query.set(q);
        }
    });

    Effect::new(move |_| {
        if let Some(window) = web_sys::window() {
            let closure = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |_| {
                set_page.set(read_current_page());
            }));
            let _ = window
                .add_event_listener_with_callback("popstate", closure.as_ref().unchecked_ref());
            closure.forget();
        }
    });

    Effect::new(move |_| {
        spawn_auth_status_load(set_auth_status, set_status);
    });

    let logout = Callback::new(move |_| {
        let req = serde_json::json!({});
        spawn_result! {
            api_post_json::<AuthStatusResponse, _>("/api/auth/logout", &req),
            Ok(updated) => {
                set_auth_status.set(Some(updated));
                set_current_track.set(None);
                set_playlist.set(Vec::new());
                set_playlist_index.set(-1);
                set_status.set(String::from("Login required"));
            },
            Err(err) => { set_status.set(err); },
        };
    });

    view! {
        {move || match_any_view!(auth_status.get(), {
            None => view! {
                <main class="auth-shell">
                    <section class="auth-panel">
                        <h1>"Easy Musiclib"</h1>
                        <p>{status}</p>
                    </section>
                </main>
            },
            Some(auth) if auth.login_required && !auth.authenticated => view! {
                <main class="auth-shell">
                    <form
                        class="auth-panel"
                        on:submit=move |ev| {
                            ev.prevent_default();
                            let username = login_username.get_untracked().trim().to_string();
                            let password = login_password.get_untracked();
                            let secure_transport = auth_status
                                .get_untracked()
                                .map(|status| status.secure_transport)
                                .unwrap_or(true);
                            let req = LoginRequest { username, password };
                            set_status.set(String::from("Signing in"));
                            spawn_result! {
                                api_post_json::<easy_musiclib_shared::LoginResponse, _>("/api/auth/login", &req),
                                Ok(login) => {
                                    set_auth_status.set(Some(AuthStatusResponse {
                                        login_required: true,
                                        authenticated: true,
                                        username: Some(login.username.clone()),
                                        secure_transport,
                                    }));
                                    set_login_password.set(String::new());
                                    set_status.set(format!("Signed in as {}", login.username));
                                },
                                Err(err) => { set_status.set(err); },
                            };
                        }
                    >
                        <h1>"Easy Musiclib"</h1>
                        <label class="setting-field">
                            <span>"Username"</span>
                            <input
                                autocomplete="username"
                                prop:value=login_username
                                on:input=move |ev| set_login_username.set(event_target_value(&ev))
                            />
                        </label>
                        <label class="setting-field">
                            <span>"Password"</span>
                            <input
                                type="password"
                                autocomplete="current-password"
                                prop:value=login_password
                                on:input=move |ev| set_login_password.set(event_target_value(&ev))
                            />
                        </label>
                        <button type="submit">"Log in"</button>
                        <p>{status}</p>
                    </form>
                </main>
            },
            _ => view! {
                <main class="app-shell">
                    <aside class="app-sidebar">
                        <div class="brand">
                            <h1>"Easy Musiclib"</h1>
                            <p>{status}</p>
                        </div>
                        <nav class="shell-nav" aria-label="Main navigation">
                            <button
                                type="button"
                                class=move || nav_class(matches!(page.get(), Page::Liked))
                                on:click=move |_| navigate.run(Page::Liked)
                            >
                                "Liked"
                            </button>
                            <button
                                type="button"
                                class=move || nav_class(matches!(page.get(), Page::Search { .. }))
                                on:click=move |_| navigate.run(Page::Search { q: String::new() })
                            >
                                "Search"
                            </button>
                            <button
                                type="button"
                                class=move || nav_class(matches!(page.get(), Page::Settings))
                                on:click=move |_| navigate.run(Page::Settings)
                            >
                                "Settings"
                            </button>
                            {move || match_any_view!(auth_status.get(), {
                                Some(auth) if auth.login_required => view! {
                                    <button
                                        type="button"
                                        class="nav-button"
                                        on:click=move |_| logout.run(())
                                    >
                                        "Logout"
                                    </button>
                                },
                                _ => view! { <span class="hidden"></span> },
                            })}
                        </nav>
                        <form
                            class="shell-search"
                            on:submit=move |ev| {
                                ev.prevent_default();
                                navigate.run(Page::Search { q: shell_query.get_untracked().trim().to_string() });
                            }
                        >
                            <input
                                placeholder="Search library"
                                prop:value=shell_query
                                on:input=move |ev| set_shell_query.set(event_target_value(&ev))
                            />
                            <button type="submit">"Go"</button>
                        </form>
                    </aside>
                    <section class="app-content">
                        {move || match_any_view!(page.get(), {
                            Page::Liked => view! { <LikedPage /> },
                            Page::Search { q } => view! { <SearchPage initial_query=q /> },
                            Page::Album { id } => view! { <AlbumPage id=id /> },
                            Page::Artist { id } => view! { <ArtistPage id=id /> },
                            Page::Event { id } => view! { <EventPage id=id /> },
                            Page::Relation { artist_id } => view! { <RelationPage artist_id=artist_id /> },
                            Page::Settings => view! { <SettingsPage /> },
                        })}
                    </section>
                    <Player />
                </main>
            },
        })}
    }
}
