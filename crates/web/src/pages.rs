use crate::api::{
    api_delete, api_get, api_patch_json, api_post_json, list_url, spawn_detail_load,
    spawn_json_status, spawn_like_patch, spawn_list_load, spawn_settings_load, spawn_text_status,
    start_scan_poll,
};
use crate::app::AppContext;
use crate::route::Page;
use crate::ui::{
    AlbumList, ArtistInlineLinks, ArtistList, EntityLink, EventList, HeroArtwork, Pager,
    RelationGraphView, TrackList,
};
use crate::util::{album_counts, album_date, paged_status, pretty_json};
use easy_musiclib_macros::{match_any_view, spawn_async, spawn_result};
use easy_musiclib_shared::{
    AccountListResponse, AccountSummary, AlbumDetail, AlbumSummary, AliasCsvImportRequest,
    AppSettings, ArtistDetail, ArtistSummary, BROWSER_PLAYBACK_FLAC_SAMPLE_RATE_OPTIONS,
    BROWSER_PLAYBACK_OPUS_BITRATE_OPTIONS, BrowserPlaybackFormat, BrowserPlaybackSettings,
    CreateAccountRequest, CreateArtistRequest, DeleteAccountResponse, EventDetail, EventSummary,
    HlsCacheClearResponse, LoginRequest, LoginResponse, MergeArtistsRequest, RelationGraph,
    ScanJobRequest, ScanJobStatus, TrackSummary, UpdateAccountPasswordRequest,
    UpdateAppSettingsRequest,
};
use leptos::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EntityKind {
    Tracks,
    Albums,
    Artists,
    Events,
}

#[component]
pub(crate) fn LikedPage() -> impl IntoView {
    let (kind, set_kind) = signal(EntityKind::Tracks);
    let (page, set_page) = signal(1_i64);
    let (total, set_total) = signal(0_i64);
    let (status, set_status) = signal(String::from("Loading"));
    let (tracks, set_tracks) = signal(Vec::<TrackSummary>::new());
    let (albums, set_albums) = signal(Vec::<AlbumSummary>::new());
    let (artists, set_artists) = signal(Vec::<ArtistSummary>::new());
    let (events, set_events) = signal(Vec::<EventSummary>::new());

    let load = move |target_kind: EntityKind, target_page: i64| {
        set_status.set(String::from("Loading"));
        match target_kind {
            EntityKind::Tracks => spawn_list_load(
                list_url("tracks", &[("liked", "true".to_string())], target_page),
                target_page,
                set_tracks,
                set_page,
                set_total,
                set_status,
                "tracks",
            ),
            EntityKind::Albums => spawn_list_load(
                list_url("albums", &[("liked", "true".to_string())], target_page),
                target_page,
                set_albums,
                set_page,
                set_total,
                set_status,
                "albums",
            ),
            EntityKind::Artists => spawn_list_load(
                list_url("artists", &[("liked", "true".to_string())], target_page),
                target_page,
                set_artists,
                set_page,
                set_total,
                set_status,
                "artists",
            ),
            EntityKind::Events => spawn_list_load(
                list_url("events", &[("liked", "true".to_string())], target_page),
                target_page,
                set_events,
                set_page,
                set_total,
                set_status,
                "events",
            ),
        }
    };

    Effect::new(move |_| {
        load(kind.get(), 1);
    });

    view! {
        <section class="page">
            <header class="page-header">
                <div>
                    <h2>"Liked"</h2>
                    <p>{move || paged_status(status.get(), page.get(), total.get())}</p>
                </div>
                <div class="tabs" role="tablist">
                    <KindTab label="Tracks" value=EntityKind::Tracks kind=kind set_kind=set_kind />
                    <KindTab label="Albums" value=EntityKind::Albums kind=kind set_kind=set_kind />
                    <KindTab label="Artists" value=EntityKind::Artists kind=kind set_kind=set_kind />
                    <KindTab label="Events" value=EntityKind::Events kind=kind set_kind=set_kind />
                </div>
            </header>
            <Pager page=page total=total on_page=Callback::new(move |next| load(kind.get_untracked(), next)) />
            {move || match_any_view!(kind.get(), {
                EntityKind::Tracks => view! { <TrackList tracks=tracks.into() /> },
                EntityKind::Albums => view! { <AlbumList albums=albums.into() /> },
                EntityKind::Artists => view! { <ArtistList artists=artists.into() /> },
                EntityKind::Events => view! { <EventList events=events.into() /> },
            })}
        </section>
    }
}

#[component]
fn KindTab(
    label: &'static str,
    value: EntityKind,
    kind: ReadSignal<EntityKind>,
    set_kind: WriteSignal<EntityKind>,
) -> impl IntoView {
    view! {
        <button
            type="button"
            role="tab"
            class=move || { if kind.get() == value { "tab active" } else { "tab" } }
            aria-selected=move || (kind.get() == value).to_string()
            on:click=move |_| set_kind.set(value)
        >
            {label}
        </button>
    }
}

#[component]
pub(crate) fn SearchPage(initial_query: String) -> impl IntoView {
    let ctx = expect_context::<AppContext>();
    let (query, set_query) = signal(initial_query.clone());
    let (status, set_status) = signal(String::from("Ready"));

    let (track_page, set_track_page) = signal(1_i64);
    let (track_total, set_track_total) = signal(0_i64);
    let (track_status, set_track_status) = signal(String::from("Ready"));
    let (tracks, set_tracks) = signal(Vec::<TrackSummary>::new());

    let (album_page, set_album_page) = signal(1_i64);
    let (album_total, set_album_total) = signal(0_i64);
    let (album_status, set_album_status) = signal(String::from("Ready"));
    let (albums, set_albums) = signal(Vec::<AlbumSummary>::new());

    let (artist_page, set_artist_page) = signal(1_i64);
    let (artist_total, set_artist_total) = signal(0_i64);
    let (artist_status, set_artist_status) = signal(String::from("Ready"));
    let (artists, set_artists) = signal(Vec::<ArtistSummary>::new());

    let (event_page, set_event_page) = signal(1_i64);
    let (event_total, set_event_total) = signal(0_i64);
    let (event_status, set_event_status) = signal(String::from("Ready"));
    let (events, set_events) = signal(Vec::<EventSummary>::new());

    let load_tracks = move |q: String, target_page: i64| {
        spawn_list_load(
            list_url("tracks", &[("q", q)], target_page),
            target_page,
            set_tracks,
            set_track_page,
            set_track_total,
            set_track_status,
            "tracks",
        );
    };
    let load_albums = move |q: String, target_page: i64| {
        spawn_list_load(
            list_url("albums", &[("q", q)], target_page),
            target_page,
            set_albums,
            set_album_page,
            set_album_total,
            set_album_status,
            "albums",
        );
    };
    let load_artists = move |q: String, target_page: i64| {
        spawn_list_load(
            list_url("artists", &[("q", q)], target_page),
            target_page,
            set_artists,
            set_artist_page,
            set_artist_total,
            set_artist_status,
            "artists",
        );
    };
    let load_events = move |q: String, target_page: i64| {
        spawn_list_load(
            list_url("events", &[("q", q)], target_page),
            target_page,
            set_events,
            set_event_page,
            set_event_total,
            set_event_status,
            "events",
        );
    };

    let run = move |update_route: bool| {
        let q = query.get_untracked().trim().to_string();
        if update_route {
            ctx.navigate.run(Page::Search { q: q.clone() });
        }
        if q.is_empty() {
            set_status.set(String::from("Ready"));
            set_tracks.set(Vec::new());
            set_albums.set(Vec::new());
            set_artists.set(Vec::new());
            set_events.set(Vec::new());
            set_track_total.set(0);
            set_album_total.set(0);
            set_artist_total.set(0);
            set_event_total.set(0);
            return;
        }
        set_status.set(format!("Search results for \"{q}\""));
        load_tracks(q.clone(), 1);
        load_albums(q.clone(), 1);
        load_artists(q.clone(), 1);
        load_events(q, 1);
    };

    Effect::new(move |_| {
        if !initial_query.trim().is_empty() {
            run(false);
        }
    });

    view! {
        <section class="page">
            <header class="page-header">
                <div>
                    <h2>"Search"</h2>
                    <p>{status}</p>
                </div>
            </header>
            <form
                class="toolbar"
                on:submit=move |ev| {
                    ev.prevent_default();
                    run(true);
                }
            >
                <input
                    autofocus
                    placeholder="Search tracks, albums, artists, events"
                    prop:value=query
                    on:input=move |ev| set_query.set(event_target_value(&ev))
                />
                <button type="submit">"Search"</button>
            </form>
            <ResultSection title="Tracks" status=track_status page=track_page total=track_total on_page=Callback::new(move |next| load_tracks(query.get_untracked(), next))>
                <TrackList tracks=tracks.into() />
            </ResultSection>
            <ResultSection title="Albums" status=album_status page=album_page total=album_total on_page=Callback::new(move |next| load_albums(query.get_untracked(), next))>
                <AlbumList albums=albums.into() />
            </ResultSection>
            <ResultSection title="Artists" status=artist_status page=artist_page total=artist_total on_page=Callback::new(move |next| load_artists(query.get_untracked(), next))>
                <ArtistList artists=artists.into() />
            </ResultSection>
            <ResultSection title="Events" status=event_status page=event_page total=event_total on_page=Callback::new(move |next| load_events(query.get_untracked(), next))>
                <EventList events=events.into() />
            </ResultSection>
        </section>
    }
}

#[component]
fn ResultSection<IV>(
    title: &'static str,
    status: ReadSignal<String>,
    page: ReadSignal<i64>,
    total: ReadSignal<i64>,
    on_page: Callback<i64>,
    children: TypedChildren<IV>,
) -> impl IntoView
where
    IV: IntoView + 'static,
{
    let children = children.into_inner();
    view! {
        <section class="result-section">
            <div class="section-header">
                <h3>{title}</h3>
                <span>{move || paged_status(status.get(), page.get(), total.get())}</span>
            </div>
            <Pager page=page total=total on_page=on_page />
            {children()}
        </section>
    }
}

#[component]
pub(crate) fn AlbumPage(id: String) -> impl IntoView {
    let ctx = expect_context::<AppContext>();
    let (detail, set_detail) = signal::<Option<AlbumDetail>>(None);
    let (status, set_status) = signal(String::from("Loading album"));

    Effect::new({
        let id = id.clone();
        move |_| {
            spawn_detail_load(
                format!("/api/albums/{}", id.clone()),
                set_detail,
                set_status,
            );
        }
    });

    let tracks = Signal::derive(move || detail.get().map(|data| data.tracks).unwrap_or_default());
    let toggle_like = move |_| {
        if let Some(album) = detail.get_untracked() {
            let liked = album.summary.liked_at.is_none();
            spawn_like_patch(
                format!("/api/albums/{}", album.summary.id),
                liked,
                set_detail,
                ctx.set_status,
            );
        }
    };

    view! {
        <section class="page detail-page">
            {move || match_any_view!(detail.get(), {
                Some(album) => {
                    let summary = album.summary.clone();
                    let date = album_date(&summary.date, summary.year);
                    let counts = album_counts(&summary);
                    view! {
                        <>
                            <HeroArtwork artwork_id=summary.artwork_id />
                            <section class="hero-copy">
                                <h2>{summary.title.clone()}</h2>
                                <p><ArtistInlineLinks artists=summary.album_artists.clone() /></p>
                                <p>
                                    {(!date.is_empty()).then(|| view! {
                                        <>
                                            {date.clone()}
                                            " · "
                                        </>
                                    })}
                                    {counts}
                                    {summary.event.clone().map(|event| view! {
                                        <>
                                            " · "
                                            <EntityLink page=Page::Event { id: event.id.to_string() } label=event.name />
                                        </>
                                    })}
                                </p>
                                <button class="like-pill" type="button" on:click=toggle_like>
                                    {if summary.liked_at.is_some() { "♥ Liked" } else { "♡ Like" }}
                                </button>
                            </section>
                            <div class="section-header"><h3>"Tracks"</h3></div>
                            <TrackList tracks=tracks show_disc_dividers=true />
                        </>
                    }
                },
                None => view! { <p class="empty">{status.get()}</p> },
            })}
        </section>
    }
}

#[component]
pub(crate) fn ArtistPage(id: String) -> impl IntoView {
    let ctx = expect_context::<AppContext>();
    let (detail, set_detail) = signal::<Option<ArtistDetail>>(None);
    let (status, set_status) = signal(String::from("Loading artist"));
    let (albums, set_albums) = signal(Vec::<AlbumSummary>::new());
    let (album_page, set_album_page) = signal(1_i64);
    let (album_total, set_album_total) = signal(0_i64);
    let (album_status, set_album_status) = signal(String::from("Loading"));
    let (tracks, set_tracks) = signal(Vec::<TrackSummary>::new());
    let (track_page, set_track_page) = signal(1_i64);
    let (track_total, set_track_total) = signal(0_i64);
    let (track_status, set_track_status) = signal(String::from("Loading"));

    let load_albums = Callback::new({
        let id = id.clone();
        move |target_page: i64| {
            spawn_list_load(
                list_url("albums", &[("artist_id", id.clone())], target_page),
                target_page,
                set_albums,
                set_album_page,
                set_album_total,
                set_album_status,
                "albums",
            );
        }
    });
    let load_tracks = Callback::new({
        let id = id.clone();
        move |target_page: i64| {
            spawn_list_load(
                list_url("tracks", &[("artist_id", id.clone())], target_page),
                target_page,
                set_tracks,
                set_track_page,
                set_track_total,
                set_track_status,
                "tracks",
            );
        }
    });

    Effect::new({
        let id = id.clone();
        move |_| {
            spawn_detail_load(
                format!("/api/artists/{}", id.clone()),
                set_detail,
                set_status,
            );
            load_albums.run(1);
            load_tracks.run(1);
        }
    });

    let toggle_like = move |_| {
        if let Some(artist) = detail.get_untracked() {
            let liked = artist.summary.liked_at.is_none();
            spawn_like_patch(
                format!("/api/artists/{}", artist.summary.id),
                liked,
                set_detail,
                ctx.set_status,
            );
        }
    };

    view! {
        <section class="page detail-page">
            {move || match_any_view!(detail.get(), {
                Some(artist) => {
                    let summary = artist.summary.clone();
                    let relation_id = summary.id.to_string();
                    view! {
                        <>
                            <HeroArtwork artwork_id=summary.artwork_id />
                            <section class="hero-copy">
                                <h2>{summary.name.clone()}</h2>
                                <p>{format!("{} albums · {} tracks", summary.album_count, summary.track_count)}</p>
                                <div class="hero-actions">
                                    <button class="like-pill" type="button" on:click=toggle_like>
                                        {if summary.liked_at.is_some() { "♥ Liked" } else { "♡ Like" }}
                                    </button>
                                    <button
                                        class="subtle-button"
                                        type="button"
                                        on:click=move |_| ctx.navigate.run(Page::Relation { artist_id: Some(relation_id.clone()) })
                                    >
                                        "Relations"
                                    </button>
                                </div>
                            </section>
                        </>
                    }
                },
                None => view! { <p class="empty">{status.get()}</p> },
            })}
            <div class="section-header">
                <h3>"Albums"</h3>
                <span>{move || paged_status(album_status.get(), album_page.get(), album_total.get())}</span>
            </div>
            <Pager page=album_page total=album_total on_page=load_albums />
            <AlbumList albums=albums.into() />
            <div class="section-header">
                <h3>"Tracks"</h3>
                <span>{move || paged_status(track_status.get(), track_page.get(), track_total.get())}</span>
            </div>
            <Pager page=track_page total=track_total on_page=load_tracks />
            <TrackList tracks=tracks.into() />
        </section>
    }
}

#[component]
pub(crate) fn EventPage(id: String) -> impl IntoView {
    let ctx = expect_context::<AppContext>();
    let (detail, set_detail) = signal::<Option<EventDetail>>(None);
    let (status, set_status) = signal(String::from("Loading event"));
    let (albums, set_albums) = signal(Vec::<AlbumSummary>::new());
    let (album_page, set_album_page) = signal(1_i64);
    let (album_total, set_album_total) = signal(0_i64);
    let (album_status, set_album_status) = signal(String::from("Loading"));

    let load_albums = Callback::new({
        let id = id.clone();
        move |target_page: i64| {
            spawn_list_load(
                list_url("albums", &[("event_id", id.clone())], target_page),
                target_page,
                set_albums,
                set_album_page,
                set_album_total,
                set_album_status,
                "albums",
            );
        }
    });

    Effect::new({
        let id = id.clone();
        move |_| {
            spawn_detail_load(
                format!("/api/events/{}", id.clone()),
                set_detail,
                set_status,
            );
            load_albums.run(1);
        }
    });

    let toggle_like = move |_| {
        if let Some(event) = detail.get_untracked() {
            let liked = event.summary.liked_at.is_none();
            spawn_like_patch(
                format!("/api/events/{}", event.summary.id),
                liked,
                set_detail,
                ctx.set_status,
            );
        }
    };

    view! {
        <section class="page detail-page">
            {move || match_any_view!(detail.get(), {
                Some(event) => {
                    let summary = event.summary.clone();
                    view! {
                        <>
                            <div class="hero-placeholder">"EVENT"</div>
                            <section class="hero-copy">
                                <h2>{summary.name.clone()}</h2>
                                <p>{album_date(&summary.date, summary.year)}</p>
                                <button class="like-pill" type="button" on:click=toggle_like>
                                    {if summary.liked_at.is_some() { "♥ Liked" } else { "♡ Like" }}
                                </button>
                            </section>
                        </>
                    }
                },
                None => view! { <p class="empty">{status.get()}</p> },
            })}
            <div class="section-header">
                <h3>"Albums"</h3>
                <span>{move || paged_status(album_status.get(), album_page.get(), album_total.get())}</span>
            </div>
            <Pager page=album_page total=album_total on_page=load_albums />
            <AlbumList albums=albums.into() />
        </section>
    }
}

#[component]
pub(crate) fn RelationPage(artist_id: Option<String>) -> impl IntoView {
    let (graph, set_graph) = signal::<Option<RelationGraph>>(None);
    let (status, set_status) = signal(String::from("Loading relations"));

    Effect::new({
        let artist_id = artist_id.clone();
        move |_| {
            let url = if let Some(id) = artist_id.clone() {
                format!(
                    "/api/relations?artist_id={}&depth=2&limit_nodes=300",
                    urlencoding::encode(&id)
                )
            } else {
                String::from("/api/relations?scope=all&limit_nodes=300")
            };
            spawn_result! {
                api_get::<RelationGraph>(&url),
                Ok(data) => {
                    set_status.set(format!(
                        "{} nodes, {} edges",
                        data.nodes.len(),
                        data.edges.len()
                    ));
                    set_graph.set(Some(data));
                },
                Err(err) => { set_status.set(err); },
            };
        }
    });

    view! {
        <section class="page">
            <header class="page-header">
                <div>
                    <h2>"Relations"</h2>
                    <p>{status}</p>
                </div>
            </header>
            {move || graph.get().map(|graph| view! { <RelationGraphView graph=graph /> })}
        </section>
    }
}

#[component]
pub(crate) fn SettingsPage() -> impl IntoView {
    let (scan_root, set_scan_root) = signal(String::from("/Volumes/smb/media"));
    let (scan_status, set_scan_status) = signal(String::new());
    let (artist_name, set_artist_name) = signal(String::new());
    let (target, set_target) = signal(String::new());
    let (source, set_source) = signal(String::new());
    let (by_name, set_by_name) = signal(false);
    let (alias_csv, set_alias_csv) = signal(String::new());
    let (settings_status, set_settings_status) = signal(String::new());
    let (browser_playback, set_browser_playback) = signal(BrowserPlaybackSettings::default());
    let (accounts, set_accounts) = signal(Vec::<AccountSummary>::new());
    let (account_username, set_account_username) = signal(String::new());
    let (account_password, set_account_password) = signal(String::new());
    let (password_account, set_password_account) = signal(None::<String>);
    let (password_update, set_password_update) = signal(String::new());

    let reload_accounts = Callback::new(move |_| {
        spawn_result! {
            api_get::<AccountListResponse>("/api/settings/accounts"),
            Ok(data) => { set_accounts.set(data.accounts); },
            Err(err) => { set_settings_status.set(err); },
        };
    });

    Effect::new(move |_| {
        spawn_settings_load(set_browser_playback, set_settings_status);
        reload_accounts.run(());
    });

    let start_scan = move |_| {
        let root = scan_root.get_untracked().trim().to_string();
        if root.is_empty() {
            set_scan_status.set(String::from("Scan root is required"));
            return;
        }
        let req = ScanJobRequest { roots: vec![root] };
        spawn_result! {
            api_post_json::<ScanJobStatus, _>("/api/scan-jobs", &req),
            Ok(job) => {
                set_scan_status.set(pretty_json(&job));
                start_scan_poll(job.id, set_scan_status);
            },
            Err(err) => { set_scan_status.set(err); },
        };
    };

    let add_artist = move |_| {
        let name = artist_name.get_untracked().trim().to_string();
        let req = CreateArtistRequest { name };
        spawn_json_status(
            async move { api_post_json::<ArtistSummary, _>("/api/artists", &req).await },
            set_settings_status,
        );
    };

    let merge = move |_| {
        let req = MergeArtistsRequest {
            target: target.get_untracked(),
            source: source.get_untracked(),
            by_name: by_name.get_untracked(),
        };
        spawn_json_status(
            async move { api_post_json::<serde_json::Value, _>("/api/artists/merge", &req).await },
            set_settings_status,
        );
    };

    let import_csv = move |_| {
        let req = AliasCsvImportRequest {
            csv: alias_csv.get_untracked(),
        };
        spawn_json_status(
            async move {
                api_post_json::<serde_json::Value, _>("/api/artists/alias-csv-import", &req).await
            },
            set_settings_status,
        );
    };

    let auto_merge = move |_| {
        let req = serde_json::json!({});
        spawn_json_status(
            async move { api_post_json::<serde_json::Value, _>("/api/artists/auto-merge", &req).await },
            set_settings_status,
        );
    };

    let save_playback = Callback::new(move |playback: BrowserPlaybackSettings| {
        let playback = playback.normalized();
        set_browser_playback.set(playback);
        set_settings_status.set(String::from("Saving playback"));
        let req = UpdateAppSettingsRequest {
            browser_playback: playback,
        };
        spawn_text_status(
            async move { api_patch_json::<AppSettings, _>("/api/settings", &req).await },
            set_settings_status,
            "Playback saved",
        );
    });

    let save_playback_format = move |format: BrowserPlaybackFormat| {
        let mut playback = browser_playback.get_untracked();
        playback.format = format;
        save_playback.run(playback);
    };

    let save_opus_bitrate = move |value: String| {
        let mut playback = browser_playback.get_untracked();
        playback.opus_bitrate = parse_i64_or(&value, playback.opus_bitrate);
        save_playback.run(playback);
    };

    let save_flac_sample_rate = move |value: String| {
        let mut playback = browser_playback.get_untracked();
        playback.flac_sample_rate = parse_i64_or(&value, playback.flac_sample_rate);
        save_playback.run(playback);
    };

    let add_account = move || {
        let username = account_username.get_untracked().trim().to_string();
        let password = account_password.get_untracked();
        if username.is_empty() || password.is_empty() {
            set_settings_status.set(String::from("Username and password are required"));
            return;
        }
        if accounts
            .get_untracked()
            .iter()
            .any(|account| account.username.eq_ignore_ascii_case(&username))
        {
            set_settings_status.set(String::from(
                "Account already exists; change its password from the account list",
            ));
            return;
        }
        let was_open = accounts.get_untracked().is_empty();
        let req = CreateAccountRequest {
            username: username.clone(),
            password: password.clone(),
        };
        set_settings_status.set(String::from("Saving account"));
        spawn_async! {
            match api_post_json::<AccountSummary, _>("/api/settings/accounts", &req).await {
                Ok(_) => {
                    set_account_username.set(String::new());
                    set_account_password.set(String::new());
                    set_password_account.set(None);
                    set_password_update.set(String::new());
                    if was_open {
                        let login_req = LoginRequest { username, password };
                        match api_post_json::<LoginResponse, _>("/api/auth/login", &login_req).await {
                            Ok(_) => set_settings_status.set(String::from("Account saved")),
                            Err(err) => set_settings_status.set(err),
                        }
                    } else {
                        set_settings_status.set(String::from("Account saved"));
                    }
                    reload_accounts.run(());
                }
                Err(err) => set_settings_status.set(err),
            }
        };
    };

    let begin_password_update = Callback::new(move |username: String| {
        set_password_account.set(Some(username));
        set_password_update.set(String::new());
    });

    let cancel_password_update = move |_| {
        set_password_account.set(None);
        set_password_update.set(String::new());
    };

    let update_password = Callback::new(move |username: String| {
        let password = password_update.get_untracked();
        if password.is_empty() {
            set_settings_status.set(String::from("New password is required"));
            return;
        }
        let req = UpdateAccountPasswordRequest { password };
        let url = format!("/api/settings/accounts/{}", urlencoding::encode(&username));
        spawn_result! {
            api_patch_json::<AccountSummary, _>(&url, &req),
            Ok(_) => {
                set_password_account.set(None);
                set_password_update.set(String::new());
                set_settings_status.set(String::from("Password updated"));
                reload_accounts.run(());
            },
            Err(err) => { set_settings_status.set(err); },
        };
    });

    let delete_account_action = Callback::new(move |username: String| {
        let url = format!("/api/settings/accounts/{}", urlencoding::encode(&username));
        spawn_result! {
            api_delete::<DeleteAccountResponse>(&url),
            Ok(_) => {
                set_password_account.set(None);
                set_password_update.set(String::new());
                set_settings_status.set(String::from("Account deleted"));
                reload_accounts.run(());
            },
            Err(err) => { set_settings_status.set(err); },
        };
    });

    let clear_hls_cache = move |_| {
        set_settings_status.set(String::from("Clearing HLS cache"));
        let req = serde_json::json!({});
        spawn_result! {
            api_post_json::<HlsCacheClearResponse, _>("/api/cache/hls/clear", &req),
            Ok(data) => {
                set_settings_status.set(format!("HLS cache cleared\n{}", pretty_json(&data)));
            },
            Err(err) => { set_settings_status.set(err); },
        };
    };

    view! {
        <section class="page settings">
            <header class="page-header">
                <div>
                    <h2>"Settings"</h2>
                    <p>"Library maintenance and artist cleanup"</p>
                </div>
            </header>
            <section class="settings-section">
                <h3>"Playback"</h3>
                <div class="settings-grid">
                    <label class="checkbox-row">
                        <input
                            type="radio"
                            name="browser-playback-format"
                            prop:checked=move || browser_playback.get().format == BrowserPlaybackFormat::Opus
                            on:change=move |_| save_playback_format(BrowserPlaybackFormat::Opus)
                        />
                        "Opus"
                    </label>
                    <label class="checkbox-row">
                        <input
                            type="radio"
                            name="browser-playback-format"
                            prop:checked=move || browser_playback.get().format == BrowserPlaybackFormat::Flac
                            on:change=move |_| save_playback_format(BrowserPlaybackFormat::Flac)
                        />
                        "FLAC"
                    </label>
                </div>
                <div class="settings-grid">
                    <label class="setting-field">
                        <span>"Opus bitrate (kbps)"</span>
                        <select
                            prop:value=move || browser_playback.get().opus_bitrate.to_string()
                            on:change=move |ev| save_opus_bitrate(event_target_value(&ev))
                        >
                            <For
                                each=move || BROWSER_PLAYBACK_OPUS_BITRATE_OPTIONS
                                key=|bitrate| *bitrate
                                children=move |bitrate| view! {
                                    <option value=bitrate.to_string()>
                                        {format!("{} kbps", bitrate / 1000)}
                                    </option>
                                }
                            />
                        </select>
                    </label>
                    <label class="setting-field">
                        <span>"FLAC sample rate (Hz)"</span>
                        <select
                            prop:value=move || browser_playback.get().flac_sample_rate.to_string()
                            on:change=move |ev| save_flac_sample_rate(event_target_value(&ev))
                        >
                            <For
                                each=move || BROWSER_PLAYBACK_FLAC_SAMPLE_RATE_OPTIONS
                                key=|sample_rate| *sample_rate
                                children=move |sample_rate| view! {
                                    <option value=sample_rate.to_string()>
                                        {format_sample_rate(sample_rate)}
                                    </option>
                                }
                            />
                        </select>
                    </label>
                </div>
                <div class="button-row">
                    <button type="button" on:click=clear_hls_cache>"Clear HLS cache"</button>
                </div>
            </section>
            <section class="settings-section">
                <h3>"Accounts"</h3>
                <form
                    class="account-create"
                    on:submit=move |ev| {
                        ev.prevent_default();
                        add_account();
                    }
                >
                    <label class="setting-field">
                        <span>"Username"</span>
                        <input
                            autocomplete="username"
                            placeholder="New username"
                            prop:value=account_username
                            on:input=move |ev| set_account_username.set(event_target_value(&ev))
                        />
                    </label>
                    <label class="setting-field">
                        <span>"Password"</span>
                        <input
                            type="password"
                            autocomplete="new-password"
                            placeholder="New password"
                            prop:value=account_password
                            on:input=move |ev| set_account_password.set(event_target_value(&ev))
                        />
                    </label>
                    <button type="submit">"Add account"</button>
                </form>
                <p class=move || if accounts.get().is_empty() { "empty account-empty" } else { "hidden" }>
                    "No accounts"
                </p>
                <div class="account-list">
                    <For
                        each=move || accounts.get()
                        key=|account| account.username.clone()
                        children=move |account| {
                            let username = account.username;
                            let display_username = username.clone();
                            let edit_username = username.clone();
                            let delete_username = username.clone();
                            let editor_username = username.clone();
                            view! {
                                <div class="account-row">
                                    <span class="account-name">{display_username}</span>
                                    <div class="account-row-actions">
                                        <button
                                            type="button"
                                            class="secondary-button"
                                            on:click=move |_| begin_password_update.run(edit_username.clone())
                                        >
                                            "Change password"
                                        </button>
                                        <button
                                            type="button"
                                            class="danger-button"
                                            on:click=move |_| delete_account_action.run(delete_username.clone())
                                        >
                                            "Delete"
                                        </button>
                                    </div>
                                    {move || {
                                        if password_account.get().as_deref() == Some(editor_username.as_str()) {
                                            let submit_username = editor_username.clone();
                                            view! {
                                                <form
                                                    class="account-password-editor"
                                                    on:submit=move |ev| {
                                                        ev.prevent_default();
                                                        update_password.run(submit_username.clone());
                                                    }
                                                >
                                                    <label class="setting-field">
                                                        <span>"New password"</span>
                                                        <input
                                                            type="password"
                                                            autocomplete="new-password"
                                                            prop:value=password_update
                                                            on:input=move |ev| set_password_update.set(event_target_value(&ev))
                                                        />
                                                    </label>
                                                    <button type="submit">"Save password"</button>
                                                    <button
                                                        type="button"
                                                        class="secondary-button"
                                                        on:click=cancel_password_update
                                                    >
                                                        "Cancel"
                                                    </button>
                                                </form>
                                            }.into_any()
                                        } else {
                                            view! { <div class="hidden"></div> }.into_any()
                                        }
                                    }}
                                </div>
                            }
                        }
                    />
                </div>
            </section>
            <section class="settings-section">
                <h3>"Scan"</h3>
                <div class="field-row">
                    <input
                        placeholder="Root path"
                        prop:value=scan_root
                        on:input=move |ev| set_scan_root.set(event_target_value(&ev))
                    />
                    <button type="button" on:click=start_scan>"Start scan"</button>
                </div>
                <pre>{scan_status}</pre>
            </section>
            <section class="settings-section">
                <h3>"Artists"</h3>
                <div class="field-row">
                    <input
                        placeholder="Artist name"
                        prop:value=artist_name
                        on:input=move |ev| set_artist_name.set(event_target_value(&ev))
                    />
                    <button type="button" on:click=add_artist>"Add artist"</button>
                </div>
                <div class="settings-grid">
                    <input
                        placeholder="Target id/uuid/name"
                        prop:value=target
                        on:input=move |ev| set_target.set(event_target_value(&ev))
                    />
                    <input
                        placeholder="Source id/uuid/name"
                        prop:value=source
                        on:input=move |ev| set_source.set(event_target_value(&ev))
                    />
                    <label class="checkbox-row">
                        <input
                            type="checkbox"
                            prop:checked=by_name
                            on:change=move |ev| set_by_name.set(event_target_checked(&ev))
                        />
                        "Merge by name"
                    </label>
                    <button type="button" on:click=merge>"Merge"</button>
                </div>
                <textarea
                    placeholder="primary,alias1,alias2"
                    prop:value=alias_csv
                    on:input=move |ev| set_alias_csv.set(event_target_value(&ev))
                ></textarea>
                <div class="button-row">
                    <button type="button" on:click=import_csv>"Import alias CSV"</button>
                    <button type="button" on:click=auto_merge>"Auto merge"</button>
                </div>
                <pre>{settings_status}</pre>
            </section>
        </section>
    }
}

fn parse_i64_or(value: &str, fallback: i64) -> i64 {
    value.trim().parse::<i64>().unwrap_or(fallback)
}

fn format_sample_rate(sample_rate: i64) -> String {
    if sample_rate % 1000 == 0 {
        format!("{} kHz", sample_rate / 1000)
    } else {
        format!("{:.1} kHz", sample_rate as f64 / 1000.0)
    }
}
