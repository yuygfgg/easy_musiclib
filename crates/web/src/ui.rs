use crate::api::api_patch_json;
use crate::app::{AppContext, PlayRequest};
use crate::hls_prefetch::spawn_hls_page_prefetch;
use crate::relation_layout::{
    GRAPH_VIEWBOX_HEIGHT, GRAPH_VIEWBOX_WIDTH, LayoutPosition, clamp_graph_position,
    layout_position, max_layout_shift, relation_graph_layout, relation_graph_layout_relax,
    relation_graph_layout_tick,
};
use crate::route::Page;
use crate::util::{album_date, playable_tracks, total_pages};
use easy_musiclib_shared::{
    AlbumSummary, ArtistSummary, EntityRef, EventSummary, Id, LikePatch, RelationEdge,
    RelationGraph, RelationNode, TrackDetail, TrackSummary,
};
use leptos::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
use wasm_bindgen::{JsCast, JsValue, prelude::Closure};

const RELAX_FRAME_LIMIT: u32 = 48;
const RELAX_STOP_SHIFT: f64 = 0.18;

#[component]
pub(crate) fn TrackList(tracks: Signal<Vec<TrackSummary>>) -> impl IntoView {
    Effect::new(move |_| {
        spawn_hls_page_prefetch(tracks.get());
    });

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
            <button
                type="button"
                disabled=move || { page.get() >= total_pages() }
                on:click=move |_| { on_page.run(page.get_untracked() + 1); }
            >
                "Next"
            </button>
            <button
                type="button"
                disabled=move || { page.get() >= total_pages() }
                on:click=move |_| { on_page.run(total_pages()); }
            >
                "Last"
            </button>
        </div>
    }
}

#[component]
pub(crate) fn RelationGraphView(graph: RelationGraph) -> impl IntoView {
    let ctx = expect_context::<AppContext>();
    let graph_nodes = graph.nodes.clone();
    let graph_edges = graph.edges.clone();
    let edge_strength_range = edge_strength_range(&graph_edges);
    let (positions, set_positions) = signal(relation_graph_layout(&graph_nodes, &graph_edges));
    let (dragging, set_dragging) = signal::<Option<Id>>(None);
    let relax_generation = Arc::new(AtomicU64::new(0));
    let nodes_for_drag = graph_nodes.clone();
    let edges_for_drag = graph_edges.clone();
    let nodes_for_mouseup = graph_nodes.clone();
    let edges_for_mouseup = graph_edges.clone();
    let relax_generation_for_mouseup = relax_generation.clone();
    let nodes_for_mouseleave = graph_nodes.clone();
    let edges_for_mouseleave = graph_edges.clone();
    let relax_generation_for_mouseleave = relax_generation.clone();

    view! {
        <svg
            class="graph"
            viewBox="0 0 1200 700"
            role="img"
            aria-label="Artist relation graph"
            on:mousemove=move |ev| {
                let Some(node_id) = dragging.get_untracked() else {
                    return;
                };
                ev.prevent_default();
                let Some((x, y)) = graph_event_position(&ev) else {
                    return;
                };
                set_positions.update(|positions| {
                    let updated = relation_graph_layout_tick(
                        &nodes_for_drag,
                        &edges_for_drag,
                        positions,
                        LayoutPosition { id: node_id, x, y },
                    );
                    *positions = updated;
                });
            }
            on:mouseup=move |ev| {
                if dragging.get_untracked().is_some() {
                    ev.prevent_default();
                    set_dragging.set(None);
                    start_layout_relax(
                        nodes_for_mouseup.clone(),
                        edges_for_mouseup.clone(),
                        set_positions,
                        dragging,
                        relax_generation_for_mouseup.clone(),
                    );
                }
            }
            on:mouseleave=move |_| {
                if dragging.get_untracked().is_some() {
                    set_dragging.set(None);
                    start_layout_relax(
                        nodes_for_mouseleave.clone(),
                        edges_for_mouseleave.clone(),
                        set_positions,
                        dragging,
                        relax_generation_for_mouseleave.clone(),
                    );
                }
            }
        >
            <For
                each=move || graph_edges.clone()
                key=|edge| (edge.source, edge.target)
                children=move |edge| {
                    let source = edge.source;
                    let target = edge.target;
                    let style = relation_edge_style(edge.strength, edge_strength_range);
                    view! {
                        <line
                            class="link"
                            x1=move || positions.with(|positions| layout_position(positions, source).0.to_string())
                            y1=move || positions.with(|positions| layout_position(positions, source).1.to_string())
                            x2=move || positions.with(|positions| layout_position(positions, target).0.to_string())
                            y2=move || positions.with(|positions| layout_position(positions, target).1.to_string())
                            stroke-width=(1_i64 + edge.strength.min(5)).to_string()
                            style=style
                        />
                    }
                }
            />
            <For
                each=move || { graph_nodes.clone() }
                key=|node| node.id
                children=move |node| {
                    let node_id = node.id;
                    let target = Page::Artist { id: node_id.to_string() };
                    let name = node.name;
                    let relax_generation_for_node = relax_generation.clone();
                    view! {
                        <g
                            class="node"
                            transform=move || positions.with(|positions| {
                                let (x, y) = layout_position(positions, node_id);
                                format!("translate({x},{y})")
                            })
                            style=move || {
                                if dragging.get() == Some(node_id) {
                                    "cursor: grabbing; user-select: none; touch-action: none;"
                                } else {
                                    "cursor: grab; user-select: none; touch-action: none;"
                                }
                            }
                            on:mousedown=move |ev| {
                                ev.prevent_default();
                                ev.stop_propagation();
                                relax_generation_for_node.fetch_add(1, Ordering::Relaxed);
                                set_dragging.set(Some(node_id));
                            }
                            on:dblclick=move |ev| {
                                ev.prevent_default();
                                ev.stop_propagation();
                                ctx.navigate.run(target.clone());
                            }
                        >
                            <circle r="9"></circle>
                            <text x="14" y="4">{name}</text>
                        </g>
                    }
                }
            />
        </svg>
    }
}

fn graph_event_position(ev: &leptos::ev::MouseEvent) -> Option<(f64, f64)> {
    let target = js_sys::Reflect::get(ev.as_ref(), &JsValue::from_str("currentTarget")).ok()?;
    if target.is_null() || target.is_undefined() {
        return None;
    }

    let rect_fn = js_sys::Reflect::get(&target, &JsValue::from_str("getBoundingClientRect"))
        .ok()?
        .dyn_into::<js_sys::Function>()
        .ok()?;
    let rect = rect_fn.call0(&target).ok()?;
    let left = js_property_number(&rect, "left")?;
    let top = js_property_number(&rect, "top")?;
    let width = js_property_number(&rect, "width")?.max(1.0);
    let height = js_property_number(&rect, "height")?.max(1.0);
    let x = (ev.client_x() as f64 - left) * GRAPH_VIEWBOX_WIDTH / width;
    let y = (ev.client_y() as f64 - top) * GRAPH_VIEWBOX_HEIGHT / height;
    Some(clamp_graph_position(x, y))
}

fn edge_strength_range(edges: &[RelationEdge]) -> (f64, f64) {
    let mut min = f64::INFINITY;
    let mut max = 0.0_f64;
    for edge in edges {
        let value = edge_strength_value(edge.strength);
        min = min.min(value);
        max = max.max(value);
    }

    if min.is_finite() && max > min {
        (min, max)
    } else {
        (0.0, 1.0)
    }
}

fn relation_edge_style(strength: i64, range: (f64, f64)) -> String {
    let t = ((edge_strength_value(strength) - range.0) / (range.1 - range.0)).clamp(0.0, 1.0);
    let (r, g, b) = if t < 0.5 {
        interpolate_color((148, 163, 184), (47, 125, 225), t * 2.0)
    } else {
        interpolate_color((47, 125, 225), (226, 85, 47), (t - 0.5) * 2.0)
    };
    let opacity = 0.42 + t * 0.48;
    format!("stroke: rgb({r}, {g}, {b}); stroke-opacity: {opacity:.2};")
}

fn edge_strength_value(strength: i64) -> f64 {
    (strength.max(1) as f64).ln_1p()
}

fn interpolate_color(start: (u8, u8, u8), end: (u8, u8, u8), t: f64) -> (u8, u8, u8) {
    let t = t.clamp(0.0, 1.0);
    (
        interpolate_channel(start.0, end.0, t),
        interpolate_channel(start.1, end.1, t),
        interpolate_channel(start.2, end.2, t),
    )
}

fn interpolate_channel(start: u8, end: u8, t: f64) -> u8 {
    (start as f64 + (end as f64 - start as f64) * t).round() as u8
}

fn start_layout_relax(
    nodes: Vec<RelationNode>,
    edges: Vec<RelationEdge>,
    set_positions: WriteSignal<Vec<LayoutPosition>>,
    dragging: ReadSignal<Option<Id>>,
    generation: Arc<AtomicU64>,
) {
    let run_id = generation.fetch_add(1, Ordering::Relaxed).wrapping_add(1);

    let nodes = Rc::new(nodes);
    let edges = Rc::new(edges);
    let frame = Rc::new(Cell::new(0_u32));
    let last_shift = Rc::new(Cell::new(f64::INFINITY));
    let callback = Rc::new(RefCell::new(None::<Closure<dyn FnMut()>>));
    let callback_for_closure = callback.clone();

    *callback.borrow_mut() = Some(Closure::<dyn FnMut()>::wrap(Box::new({
        let nodes = nodes.clone();
        let edges = edges.clone();
        let frame = frame.clone();
        let last_shift = last_shift.clone();
        let generation = generation.clone();
        move || {
            if generation.load(Ordering::Relaxed) != run_id || dragging.get_untracked().is_some() {
                callback_for_closure.borrow_mut().take();
                return;
            }

            let current_frame = frame.get();
            if current_frame >= RELAX_FRAME_LIMIT || last_shift.get() <= RELAX_STOP_SHIFT {
                callback_for_closure.borrow_mut().take();
                return;
            }

            frame.set(current_frame + 1);
            set_positions.update(|positions| {
                let updated = relation_graph_layout_relax(&nodes, &edges, positions, current_frame);
                last_shift.set(max_layout_shift(positions, &updated));
                *positions = updated;
            });

            if frame.get() >= RELAX_FRAME_LIMIT || last_shift.get() <= RELAX_STOP_SHIFT {
                callback_for_closure.borrow_mut().take();
                return;
            }

            if let Some(callback) = callback_for_closure.borrow().as_ref() {
                request_animation_frame(callback);
            }
        }
    })));

    if let Some(callback) = callback.borrow().as_ref() {
        request_animation_frame(callback);
    }
}

fn request_animation_frame(callback: &Closure<dyn FnMut()>) {
    if let Some(window) = web_sys::window() {
        let _ = window.request_animation_frame(callback.as_ref().unchecked_ref());
    }
}

fn js_property_number(value: &JsValue, key: &str) -> Option<f64> {
    js_sys::Reflect::get(value, &JsValue::from_str(key))
        .ok()?
        .as_f64()
}
