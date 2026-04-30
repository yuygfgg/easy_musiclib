use crate::api::{
    api_get, api_patch_json, api_post_json, list_url, spawn_list_load, start_scan_poll,
};
use crate::app::AppContext;
use crate::route::Page;
use crate::ui::{
    AlbumList, ArtistInlineLinks, ArtistList, EntityLink, EventList, HeroArtwork, Pager,
    RelationGraphView, TrackList,
};
use crate::util::{album_date, paged_status, pretty_json};
use easy_musiclib_shared::{
    AlbumDetail, AlbumSummary, AliasCsvImportRequest, ArtistDetail, ArtistSummary,
    CreateArtistRequest, EventDetail, EventSummary, LikePatch, MergeArtistsRequest, RelationGraph,
    ScanJobRequest, ScanJobStatus, TrackSummary,
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
            {move || match kind.get() {
                EntityKind::Tracks => view! { <TrackList tracks=tracks.into() /> }.into_any(),
                EntityKind::Albums => view! { <AlbumList albums=albums.into() /> }.into_any(),
                EntityKind::Artists => view! { <ArtistList artists=artists.into() /> }.into_any(),
                EntityKind::Events => view! { <EventList events=events.into() /> }.into_any(),
            }}
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
            let id = id.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match api_get::<AlbumDetail>(&format!("/api/albums/{id}")).await {
                    Ok(data) => set_detail.set(Some(data)),
                    Err(err) => set_status.set(err),
                }
            });
        }
    });

    let tracks = Signal::derive(move || detail.get().map(|data| data.tracks).unwrap_or_default());
    let toggle_like = move |_| {
        if let Some(album) = detail.get_untracked() {
            let liked = album.summary.liked_at.is_none();
            wasm_bindgen_futures::spawn_local(async move {
                match api_patch_json::<AlbumDetail, _>(
                    &format!("/api/albums/{}", album.summary.id),
                    &LikePatch { liked },
                )
                .await
                {
                    Ok(updated) => set_detail.set(Some(updated)),
                    Err(err) => ctx.set_status.set(err),
                }
            });
        }
    };

    view! {
        <section class="page detail-page">
            {move || match detail.get() {
                Some(album) => {
                    let summary = album.summary.clone();
                    view! {
                        <>
                            <HeroArtwork artwork_id=summary.artwork_id />
                            <section class="hero-copy">
                                <h2>{summary.title.clone()}</h2>
                                <p><ArtistInlineLinks artists=summary.album_artists.clone() /></p>
                                <p>
                                    {album_date(&summary.date, summary.year)}
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
                            <TrackList tracks=tracks />
                        </>
                    }.into_any()
                }
                None => view! { <p class="empty">{status.get()}</p> }.into_any(),
            }}
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
            let id = id.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match api_get::<ArtistDetail>(&format!("/api/artists/{id}")).await {
                    Ok(data) => set_detail.set(Some(data)),
                    Err(err) => set_status.set(err),
                }
            });
            load_albums.run(1);
            load_tracks.run(1);
        }
    });

    let toggle_like = move |_| {
        if let Some(artist) = detail.get_untracked() {
            let liked = artist.summary.liked_at.is_none();
            wasm_bindgen_futures::spawn_local(async move {
                match api_patch_json::<ArtistDetail, _>(
                    &format!("/api/artists/{}", artist.summary.id),
                    &LikePatch { liked },
                )
                .await
                {
                    Ok(updated) => set_detail.set(Some(updated)),
                    Err(err) => ctx.set_status.set(err),
                }
            });
        }
    };

    view! {
        <section class="page detail-page">
            {move || match detail.get() {
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
                    }.into_any()
                }
                None => view! { <p class="empty">{status.get()}</p> }.into_any(),
            }}
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
            let id = id.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match api_get::<EventDetail>(&format!("/api/events/{id}")).await {
                    Ok(data) => set_detail.set(Some(data)),
                    Err(err) => set_status.set(err),
                }
            });
            load_albums.run(1);
        }
    });

    let toggle_like = move |_| {
        if let Some(event) = detail.get_untracked() {
            let liked = event.summary.liked_at.is_none();
            wasm_bindgen_futures::spawn_local(async move {
                match api_patch_json::<EventDetail, _>(
                    &format!("/api/events/{}", event.summary.id),
                    &LikePatch { liked },
                )
                .await
                {
                    Ok(updated) => set_detail.set(Some(updated)),
                    Err(err) => ctx.set_status.set(err),
                }
            });
        }
    };

    view! {
        <section class="page detail-page">
            {move || match detail.get() {
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
                    }.into_any()
                }
                None => view! { <p class="empty">{status.get()}</p> }.into_any(),
            }}
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
            wasm_bindgen_futures::spawn_local(async move {
                match api_get::<RelationGraph>(&url).await {
                    Ok(data) => {
                        set_status.set(format!(
                            "{} nodes, {} edges",
                            data.nodes.len(),
                            data.edges.len()
                        ));
                        set_graph.set(Some(data));
                    }
                    Err(err) => set_status.set(err),
                }
            });
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

    let start_scan = move |_| {
        let root = scan_root.get_untracked().trim().to_string();
        if root.is_empty() {
            set_scan_status.set(String::from("Scan root is required"));
            return;
        }
        wasm_bindgen_futures::spawn_local(async move {
            match api_post_json::<ScanJobStatus, _>(
                "/api/scan-jobs",
                &ScanJobRequest { roots: vec![root] },
            )
            .await
            {
                Ok(job) => {
                    set_scan_status.set(pretty_json(&job));
                    start_scan_poll(job.id, set_scan_status);
                }
                Err(err) => set_scan_status.set(err),
            }
        });
    };

    let add_artist = move |_| {
        let name = artist_name.get_untracked().trim().to_string();
        wasm_bindgen_futures::spawn_local(async move {
            match api_post_json::<ArtistSummary, _>("/api/artists", &CreateArtistRequest { name })
                .await
            {
                Ok(data) => set_settings_status.set(pretty_json(&data)),
                Err(err) => set_settings_status.set(err),
            }
        });
    };

    let merge = move |_| {
        let req = MergeArtistsRequest {
            target: target.get_untracked(),
            source: source.get_untracked(),
            by_name: by_name.get_untracked(),
        };
        wasm_bindgen_futures::spawn_local(async move {
            match api_post_json::<serde_json::Value, _>("/api/artists/merge", &req).await {
                Ok(data) => set_settings_status.set(pretty_json(&data)),
                Err(err) => set_settings_status.set(err),
            }
        });
    };

    let import_csv = move |_| {
        let req = AliasCsvImportRequest {
            csv: alias_csv.get_untracked(),
        };
        wasm_bindgen_futures::spawn_local(async move {
            match api_post_json::<serde_json::Value, _>("/api/artists/alias-csv-import", &req).await
            {
                Ok(data) => set_settings_status.set(pretty_json(&data)),
                Err(err) => set_settings_status.set(err),
            }
        });
    };

    let auto_merge = move |_| {
        wasm_bindgen_futures::spawn_local(async move {
            match api_post_json::<serde_json::Value, _>(
                "/api/artists/auto-merge",
                &serde_json::json!({}),
            )
            .await
            {
                Ok(data) => set_settings_status.set(pretty_json(&data)),
                Err(err) => set_settings_status.set(err),
            }
        });
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
