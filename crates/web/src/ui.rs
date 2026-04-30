use crate::api::api_patch_json;
use crate::app::{AppContext, PlayRequest};
use crate::route::Page;
use crate::util::{album_date, circular_position, node_position, playable_tracks, total_pages};
use easy_musiclib_shared::{
    AlbumSummary, ArtistSummary, EntityRef, EventSummary, Id, LikePatch, RelationGraph,
    TrackDetail, TrackSummary,
};
use leptos::prelude::*;

#[component]
pub(crate) fn TrackList(tracks: Signal<Vec<TrackSummary>>) -> impl IntoView {
    view! {
        <div class="track-list">
            <For
                each=move || { tracks.get().into_iter().enumerate().collect::<Vec<_>>() }
                key=|(index, track)| (*index, track.id)
                children=move |(index, track)| view! { <TrackItem track=track index=index tracks=tracks /> }
            />
        </div>
    }
}

#[component]
fn TrackItem(
    track: TrackSummary,
    index: usize,
    tracks: Signal<Vec<TrackSummary>>,
) -> impl IntoView {
    let ctx = expect_context::<AppContext>();
    let track_id = track.id;
    let playable = track.playable;
    let (liked, set_liked) = signal(track.liked_at.is_some());

    Effect::new(move |_| {
        if let Some(updated) = ctx.track_update.get() {
            if updated.id == track_id {
                set_liked.set(updated.liked_at.is_some());
            }
        }
    });

    let play_track = {
        let track = track.clone();
        move |_| {
            if !playable {
                ctx.set_status.set(String::from("Track is not playable"));
                return;
            }
            let list = playable_tracks(tracks.get_untracked());
            let play_index = list
                .iter()
                .position(|item| item.id == track_id)
                .unwrap_or(index.min(list.len().saturating_sub(1)));
            if list.is_empty() {
                ctx.set_status.set(String::from("No playable tracks"));
                return;
            }
            ctx.set_playlist.set(list);
            ctx.set_playlist_index.set(play_index as i64);
            ctx.set_play_request.set(Some(PlayRequest {
                track: track.clone(),
            }));
        }
    };

    let toggle_like = move |ev: leptos::ev::MouseEvent| {
        ev.stop_propagation();
        let liked_next = !liked.get_untracked();
        wasm_bindgen_futures::spawn_local(async move {
            match api_patch_json::<TrackDetail, _>(
                &format!("/api/tracks/{track_id}"),
                &LikePatch { liked: liked_next },
            )
            .await
            {
                Ok(updated) => {
                    let summary = updated.summary;
                    set_liked.set(summary.liked_at.is_some());
                    ctx.set_track_update.set(Some(summary));
                }
                Err(err) => ctx.set_status.set(err),
            }
        });
    };

    view! {
        <article
            class=move || {
                let playing = ctx.current_track.get().map(|item| item.id == track_id).unwrap_or(false);
                format!(
                    "track{}{}",
                    if playing { " playing" } else { "" },
                    if playable { "" } else { " unavailable-track" },
                )
            }
            on:click=play_track
        >
            <Artwork artwork_id=track.artwork_id size=160 />
            <div class="meta">
                <strong>{track.title.clone()}</strong>
                <span><ArtistInlineLinks artists=track.artists.clone() /></span>
                <small>{track.album.clone().map(|album| view! { <EntityLink page=Page::Album { id: album.id.to_string() } label=album.name /> })}</small>
            </div>
            <div class="actions" on:click=|ev| ev.stop_propagation()>
                {(!playable).then(|| view! { <span class="unavailable">"Unavailable"</span> })}
                <button class="icon-btn" type="button" title="Like" on:click=toggle_like>
                    {move || if liked.get() { "♥" } else { "♡" }}
                </button>
                <a class="icon-btn" href=format!("/api/tracks/{}/download", track.id) title="Download" download>
                    "↓"
                </a>
            </div>
        </article>
    }
}

#[component]
pub(crate) fn AlbumList(albums: Signal<Vec<AlbumSummary>>) -> impl IntoView {
    view! {
        <div class="entity-list">
            <For
                each=move || albums.get()
                key=|album| album.id
                children=move |album| view! { <AlbumCard album=album /> }
            />
        </div>
    }
}

#[component]
fn AlbumCard(album: AlbumSummary) -> impl IntoView {
    let ctx = expect_context::<AppContext>();
    let target = Page::Album {
        id: album.id.to_string(),
    };
    view! {
        <article class="entity clickable" on:click=move |_| ctx.navigate.run(target.clone())>
            <Artwork artwork_id=album.artwork_id size=160 />
            <div class="meta">
                <strong>{album.title.clone()}</strong>
                <span><ArtistInlineLinks artists=album.album_artists.clone() /></span>
                <small>{format!("{} · {} tracks", album_date(&album.date, album.year), album.song_count)}</small>
            </div>
            <div class="entity-mark">{if album.liked_at.is_some() { "♥" } else { "" }}</div>
        </article>
    }
}

#[component]
pub(crate) fn ArtistList(artists: Signal<Vec<ArtistSummary>>) -> impl IntoView {
    view! {
        <div class="entity-list">
            <For
                each=move || artists.get()
                key=|artist| artist.id
                children=move |artist| view! { <ArtistCard artist=artist /> }
            />
        </div>
    }
}

#[component]
fn ArtistCard(artist: ArtistSummary) -> impl IntoView {
    let ctx = expect_context::<AppContext>();
    let target = Page::Artist {
        id: artist.id.to_string(),
    };
    view! {
        <article class="entity clickable" on:click=move |_| ctx.navigate.run(target.clone())>
            <Artwork artwork_id=artist.artwork_id size=160 />
            <div class="meta">
                <strong>{artist.name.clone()}</strong>
                <span>{format!("{} albums · {} tracks", artist.album_count, artist.track_count)}</span>
            </div>
            <div class="entity-mark">{if artist.liked_at.is_some() { "♥" } else { "" }}</div>
        </article>
    }
}

#[component]
pub(crate) fn EventList(events: Signal<Vec<EventSummary>>) -> impl IntoView {
    view! {
        <div class="entity-list">
            <For
                each=move || events.get()
                key=|event| event.id
                children=move |event| view! { <EventCard event=event /> }
            />
        </div>
    }
}

#[component]
fn EventCard(event: EventSummary) -> impl IntoView {
    let ctx = expect_context::<AppContext>();
    let target = Page::Event {
        id: event.id.to_string(),
    };
    view! {
        <article class="entity clickable" on:click=move |_| ctx.navigate.run(target.clone())>
            <div class="art event-art">"EV"</div>
            <div class="meta">
                <strong>{event.name.clone()}</strong>
                <span>{album_date(&event.date, event.year)}</span>
                <small>{format!("{} albums", event.album_count)}</small>
            </div>
            <div class="entity-mark">{if event.liked_at.is_some() { "♥" } else { "" }}</div>
        </article>
    }
}

#[component]
pub(crate) fn ArtistInlineLinks(artists: Vec<EntityRef>) -> impl IntoView {
    view! {
        <For
            each=move || { artists.clone().into_iter().enumerate().collect::<Vec<_>>() }
            key=|(index, artist)| (*index, artist.id)
            children=move |(index, artist)| view! {
                <span>
                    {if index > 0 { ", " } else { "" }}
                    <EntityLink page=Page::Artist { id: artist.id.to_string() } label=artist.name />
                </span>
            }
        />
    }
}

#[component]
pub(crate) fn EntityLink(page: Page, label: String) -> impl IntoView {
    let ctx = expect_context::<AppContext>();
    let href = page.to_path();
    view! {
        <a
            href=href
            on:click=move |ev| {
                ev.prevent_default();
                ev.stop_propagation();
                ctx.navigate.run(page.clone());
            }
        >
            {label}
        </a>
    }
}

#[component]
pub(crate) fn Artwork(artwork_id: Option<Id>, size: i64) -> impl IntoView {
    view! {
        <div class="art">
            {artwork_id.map(|id| view! {
                <img src=format!("/api/artwork/{id}?size={size}") loading="lazy" alt="" />
            })}
        </div>
    }
}

#[component]
pub(crate) fn HeroArtwork(artwork_id: Option<Id>) -> impl IntoView {
    view! {
        <div class="hero-art">
            {artwork_id.map(|id| view! {
                <img src=format!("/api/artwork/{id}?size=512") loading="lazy" alt="" />
            })}
        </div>
    }
}

#[component]
pub(crate) fn Pager(
    page: ReadSignal<i64>,
    total: ReadSignal<i64>,
    on_page: Callback<i64>,
) -> impl IntoView {
    let (input, set_input) = signal(String::from("1"));
    Effect::new(move |_| {
        set_input.set(page.get().to_string());
    });
    let total_pages = move || total_pages(total.get());
    let go = move || {
        let parsed = input.get_untracked().parse::<i64>().unwrap_or(1);
        on_page.run(parsed.clamp(1, total_pages()));
    };

    view! {
        <div class="pagination" class:hidden=move || total_pages() <= 1>
            <button type="button" disabled=move || page.get() <= 1 on:click=move |_| on_page.run(1)>"First"</button>
            <button type="button" disabled=move || page.get() <= 1 on:click=move |_| on_page.run(page.get_untracked() - 1)>"Prev"</button>
            <label>
                "Page"
                <input
                    type="number"
                    min="1"
                    max=move || total_pages().to_string()
                    prop:value=input
                    on:input=move |ev| set_input.set(event_target_value(&ev))
                    on:keydown=move |ev| {
                        if ev.key() == "Enter" {
                            go();
                        }
                    }
                />
                <span>{move || format!("/ {}", total_pages())}</span>
            </label>
            <button type="button" on:click=move |_| go()>"Go"</button>
            <button type="button" disabled=move || page.get() >= total_pages() on:click=move |_| on_page.run(page.get_untracked() + 1)>"Next"</button>
            <button type="button" disabled=move || page.get() >= total_pages() on:click=move |_| on_page.run(total_pages())>"Last"</button>
        </div>
    }
}

#[component]
pub(crate) fn RelationGraphView(graph: RelationGraph) -> impl IntoView {
    let ctx = expect_context::<AppContext>();
    let nodes_for_edges = graph.nodes.clone();
    let nodes_for_nodes = graph.nodes.clone();
    let node_count = graph.nodes.len().max(1);

    view! {
        <svg class="graph" viewBox="0 0 1200 700" role="img" aria-label="Artist relation graph">
            <For
                each=move || graph.edges.clone()
                key=|edge| (edge.source, edge.target)
                children=move |edge| {
                    let (x1, y1) = node_position(&nodes_for_edges, edge.source);
                    let (x2, y2) = node_position(&nodes_for_edges, edge.target);
                    view! {
                        <line
                            class="link"
                            x1=x1.to_string()
                            y1=y1.to_string()
                            x2=x2.to_string()
                            y2=y2.to_string()
                            stroke-width=(1_i64 + edge.strength.min(5)).to_string()
                        />
                    }
                }
            />
            <For
                each=move || { nodes_for_nodes.clone().into_iter().enumerate().collect::<Vec<_>>() }
                key=|(_, node)| node.id
                children=move |(index, node)| {
                    let (x, y) = circular_position(index, node_count);
                    let target = Page::Artist { id: node.id.to_string() };
                    view! {
                        <g class="node" transform=format!("translate({x},{y})") on:click=move |_| ctx.navigate.run(target.clone())>
                            <circle r="9"></circle>
                            <text x="14" y="4">{node.name}</text>
                        </g>
                    }
                }
            />
        </svg>
    }
}
