use crate::api::api_patch_json;
use crate::app::{AppContext, PlayRequest};
use crate::hls_prefetch::spawn_hls_page_prefetch;
use crate::relation_layout::{
    GRAPH_VIEWBOX_HEIGHT, GRAPH_VIEWBOX_WIDTH, LayoutBounds, LayoutPosition, layout_position,
    max_layout_shift, relation_graph_bounds_with_scale, relation_graph_layout_relax,
    relation_graph_layout_settled, relation_graph_layout_tick, relation_node_label_text,
    relation_node_label_width,
};
use crate::route::Page;
use crate::util::{album_counts, album_date, playable_tracks, total_pages};
use easy_musiclib_macros::{
    entity_list_component, js_function, js_get, match_any_view, spawn_result,
};
use easy_musiclib_shared::{
    AlbumSummary, ArtistSummary, EntityRef, EventSummary, Id, LikePatch, RelationEdge,
    RelationGraph, RelationNode, TrackDetail, TrackSummary,
};
use leptos::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
use wasm_bindgen::{JsCast, JsValue, prelude::Closure};

const RELAX_FRAME_LIMIT: u32 = 48;
const RELAX_STOP_SHIFT: f64 = 0.18;
const DOUBLE_TAP_MAX_MS: f64 = 560.0;
const DOUBLE_TAP_MAX_DISTANCE_PX: f64 = 96.0;
const TAP_MAX_MOVE_PX: f64 = 22.0;
const GRAPH_FIT_PADDING: f64 = 76.0;
const MIN_VIEWPORT_WIDTH: f64 = 160.0;
const ZOOM_IN_FACTOR: f64 = 0.78;
const ZOOM_OUT_FACTOR: f64 = 1.28;

#[derive(Clone, Copy)]
struct GraphDragStart {
    id: Id,
    pointer_id: i32,
    client_x: f64,
    client_y: f64,
}

#[derive(Clone, Copy)]
struct GraphTap {
    id: Id,
    time_ms: f64,
    client_x: f64,
    client_y: f64,
}

#[derive(Clone, Copy)]
struct GraphViewport {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl GraphViewport {
    fn view_box(self) -> String {
        format!("{} {} {} {}", self.x, self.y, self.width, self.height)
    }

    fn center(self) -> (f64, f64) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    fn node_scale(self) -> f64 {
        (self.width / GRAPH_VIEWBOX_WIDTH).max(0.001)
    }
}

#[derive(Clone, Copy)]
struct GraphPointer {
    pointer_id: i32,
    client_x: f64,
    client_y: f64,
}

#[derive(Clone, Copy)]
struct GraphPanStart {
    pointer_id: i32,
    client_x: f64,
    client_y: f64,
    viewport: GraphViewport,
}

#[derive(Clone, Copy)]
struct GraphPinchStart {
    distance: f64,
    center_x: f64,
    center_y: f64,
    viewport: GraphViewport,
}

struct GraphGesture {
    pointers: Vec<GraphPointer>,
    pan: Option<GraphPanStart>,
    pinch: Option<GraphPinchStart>,
}

impl GraphGesture {
    fn new() -> Self {
        Self {
            pointers: Vec::new(),
            pan: None,
            pinch: None,
        }
    }
}

#[component]
pub(crate) fn TrackList(
    tracks: Signal<Vec<TrackSummary>>,
    #[prop(optional)] show_disc_dividers: bool,
) -> impl IntoView {
    Effect::new(move |_| {
        spawn_hls_page_prefetch(tracks.get());
    });

    view! {
        <div class="track-list">
            <For
                each=move || track_list_rows(tracks.get(), show_disc_dividers)
                key=|row| row.key()
                children=move |row| match_any_view!(row, {
                    TrackListRow::DiscDivider { disc_no, .. } => view! {
                        <div class="disc-divider">{format!("disc{disc_no}")}</div>
                    },
                    TrackListRow::Track { index, track } => view! {
                        <TrackItem track=track index=index tracks=tracks />
                    },
                })
            />
        </div>
    }
}

#[derive(Clone)]
enum TrackListRow {
    DiscDivider { disc_no: i64, index: usize },
    Track { index: usize, track: TrackSummary },
}

impl TrackListRow {
    fn key(&self) -> String {
        match self {
            Self::DiscDivider { disc_no, index } => format!("disc-{disc_no}-{index}"),
            Self::Track { track, .. } => format!("track-{}", track.id),
        }
    }
}

fn track_list_rows(tracks: Vec<TrackSummary>, show_disc_dividers: bool) -> Vec<TrackListRow> {
    let show_disc_dividers =
        show_disc_dividers && has_multiple_discs(tracks.iter().map(track_disc_no));
    let mut rows = Vec::with_capacity(tracks.len() * if show_disc_dividers { 2 } else { 1 });
    let mut current_disc = None;
    for (index, track) in tracks.into_iter().enumerate() {
        let disc_no = track_disc_no(&track);
        if show_disc_dividers && current_disc != Some(disc_no) {
            rows.push(TrackListRow::DiscDivider { disc_no, index });
            current_disc = Some(disc_no);
        }
        rows.push(TrackListRow::Track { index, track });
    }
    rows
}

fn has_multiple_discs(mut disc_numbers: impl Iterator<Item = i64>) -> bool {
    let Some(first) = disc_numbers.next() else {
        return false;
    };
    disc_numbers.any(|disc_no| disc_no != first)
}

fn track_disc_no(track: &TrackSummary) -> i64 {
    track.disc_no.filter(|disc_no| *disc_no > 0).unwrap_or(1)
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
        spawn_result! {
            api_patch_json::<TrackDetail, _>(&format!("/api/tracks/{track_id}"), &LikePatch { liked: liked_next }),
            Ok(updated) => {
                let summary = updated.summary;
                set_liked.set(summary.liked_at.is_some());
                ctx.set_track_update.set(Some(summary));
            },
            Err(err) => { ctx.set_status.set(err); },
        };
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

entity_list_component! {
    pub(crate) fn AlbumList(albums: AlbumSummary) {
        class: "entity-list",
        key: |album| album.id,
        card: AlbumCard(album)
    }
}

#[component]
fn AlbumCard(album: AlbumSummary) -> impl IntoView {
    let ctx = expect_context::<AppContext>();
    let target = Page::Album {
        id: album.id.to_string(),
    };
    let counts = album_counts(&album);
    view! {
        <article class="entity clickable" on:click=move |_| ctx.navigate.run(target.clone())>
            <Artwork artwork_id=album.artwork_id size=160 />
            <div class="meta">
                <strong>{album.title.clone()}</strong>
                <span><ArtistInlineLinks artists=album.album_artists.clone() /></span>
                <small>{format!("{} · {counts}", album_date(&album.date, album.year))}</small>
            </div>
            <div class="entity-mark">{if album.liked_at.is_some() { "♥" } else { "" }}</div>
        </article>
    }
}

entity_list_component! {
    pub(crate) fn ArtistList(artists: ArtistSummary) {
        class: "entity-list",
        key: |artist| artist.id,
        card: ArtistCard(artist)
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

entity_list_component! {
    pub(crate) fn EventList(events: EventSummary) {
        class: "entity-list",
        key: |event| event.id,
        card: EventCard(event)
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
    let initial_positions = relation_graph_layout_settled(&graph_nodes, &graph_edges);
    let (positions, set_positions) = signal(initial_positions);
    let (viewport, set_viewport) = signal(fit_graph_viewport_for_positions(
        &graph_nodes,
        &positions.get_untracked(),
    ));
    let (dragging, set_dragging) = signal::<Option<Id>>(None);
    let relax_generation = Arc::new(AtomicU64::new(0));
    let pointer_drag = Arc::new(Mutex::new(None::<GraphDragStart>));
    let graph_gesture = Arc::new(Mutex::new(GraphGesture::new()));
    let last_tap = Arc::new(Mutex::new(None::<GraphTap>));
    let nodes_for_fit = graph_nodes.clone();
    let nodes_for_drag = graph_nodes.clone();
    let edges_for_drag = graph_edges.clone();
    let pointer_drag_for_move = pointer_drag.clone();
    let graph_gesture_for_move = graph_gesture.clone();
    let nodes_for_pointerup = graph_nodes.clone();
    let edges_for_pointerup = graph_edges.clone();
    let relax_generation_for_pointerup = relax_generation.clone();
    let pointer_drag_for_pointerup = pointer_drag.clone();
    let graph_gesture_for_pointerup = graph_gesture.clone();
    let nodes_for_pointerleave = graph_nodes.clone();
    let edges_for_pointerleave = graph_edges.clone();
    let relax_generation_for_pointerleave = relax_generation.clone();
    let pointer_drag_for_pointerleave = pointer_drag.clone();
    let graph_gesture_for_pointerleave = graph_gesture.clone();
    let nodes_for_pointercancel = graph_nodes.clone();
    let edges_for_pointercancel = graph_edges.clone();
    let relax_generation_for_pointercancel = relax_generation.clone();
    let pointer_drag_for_pointercancel = pointer_drag.clone();
    let graph_gesture_for_pointercancel = graph_gesture.clone();
    let graph_gesture_for_pointerdown = graph_gesture.clone();
    let pointer_drag_for_pointerdown = pointer_drag.clone();
    let graph_edges_for_edges = graph_edges.clone();
    let graph_nodes_for_nodes = graph_nodes.clone();
    let graph_edges_for_minimap = graph_edges.clone();
    let graph_nodes_for_minimap = graph_nodes.clone();

    view! {
        <div class="graph-shell">
            <div class="graph-controls" aria-label="Graph zoom controls">
                <button
                    type="button"
                    title="Zoom in"
                    on:click=move |_| {
                        let center = viewport.get_untracked().center();
                        set_viewport.set(zoom_graph_viewport(
                            viewport.get_untracked(),
                            center,
                            ZOOM_IN_FACTOR,
                        ));
                    }
                >
                    "+"
                </button>
                <button
                    type="button"
                    title="Zoom out"
                    on:click=move |_| {
                        let center = viewport.get_untracked().center();
                        set_viewport.set(zoom_graph_viewport(
                            viewport.get_untracked(),
                            center,
                            ZOOM_OUT_FACTOR,
                        ));
                    }
                >
                    "-"
                </button>
                <button
                    type="button"
                    title="Fit graph"
                    on:click=move |_| {
                        let next_viewport = positions.with_untracked(|positions| {
                            fit_graph_viewport_for_positions(&nodes_for_fit, positions)
                        });
                        set_viewport.set(next_viewport);
                    }
                >
                    "Fit"
                </button>
            </div>
            <svg
                class="graph"
                viewBox=move || viewport.get().view_box()
                role="img"
                aria-label="Artist relation graph"
                on:wheel=move |ev| {
                    ev.prevent_default();
                    let factor = if ev.delta_y() < 0.0 { ZOOM_IN_FACTOR } else { ZOOM_OUT_FACTOR };
                    if let Some(center) = graph_wheel_position(&ev, viewport.get_untracked()) {
                        set_viewport.set(zoom_graph_viewport(viewport.get_untracked(), center, factor));
                    }
                }
                on:pointerdown=move |ev| {
                    ev.prevent_default();
                    capture_pointer(ev.as_ref(), ev.pointer_id());
                    if ev.pointer_type() == "touch" {
                        if let Some(drag_start) = get_active_drag(&pointer_drag_for_pointerdown) {
                            set_active_drag(&pointer_drag_for_pointerdown, None);
                            set_dragging.set(None);
                            let current_viewport = viewport.get_untracked();
                            graph_gesture_pointer_down(
                                &graph_gesture_for_pointerdown,
                                drag_start.pointer_id,
                                drag_start.client_x,
                                drag_start.client_y,
                                current_viewport,
                            );
                            graph_gesture_pointer_down(
                                &graph_gesture_for_pointerdown,
                                ev.pointer_id(),
                                ev.client_x() as f64,
                                ev.client_y() as f64,
                                current_viewport,
                            );
                            return;
                        }
                    }
                    graph_gesture_pointer_down(
                        &graph_gesture_for_pointerdown,
                        ev.pointer_id(),
                        ev.client_x() as f64,
                        ev.client_y() as f64,
                        viewport.get_untracked(),
                    );
                }
                on:pointermove=move |ev| {
                    if let Some(drag_start) = active_drag(&pointer_drag_for_move, ev.pointer_id()) {
                        ev.prevent_default();
                        let Some((x, y)) = graph_pointer_position(&ev, viewport.get_untracked()) else {
                            return;
                        };
                        set_positions.update(|positions| {
                            let updated = relation_graph_layout_tick(
                                &nodes_for_drag,
                                &edges_for_drag,
                                positions,
                                LayoutPosition { id: drag_start.id, x, y },
                            );
                            *positions = updated;
                        });
                        return;
                    }

                    if let Some(next_viewport) = graph_gesture_pointer_move(
                        &graph_gesture_for_move,
                        ev.as_ref(),
                        ev.pointer_id(),
                        ev.client_x() as f64,
                        ev.client_y() as f64,
                    ) {
                        ev.prevent_default();
                        set_viewport.set(next_viewport);
                    }
                }
                on:pointerup=move |ev| {
                    if active_drag(&pointer_drag_for_pointerup, ev.pointer_id()).is_some() {
                        finish_graph_drag(
                            ev.pointer_id(),
                            &pointer_drag_for_pointerup,
                            set_dragging,
                            nodes_for_pointerup.clone(),
                            edges_for_pointerup.clone(),
                            set_positions,
                            dragging,
                            relax_generation_for_pointerup.clone(),
                            true,
                            &ev,
                        );
                    } else {
                        graph_gesture_pointer_up(&graph_gesture_for_pointerup, ev.pointer_id());
                    }
                }
                on:pointerleave=move |ev| {
                    if active_drag(&pointer_drag_for_pointerleave, ev.pointer_id()).is_some() {
                        finish_graph_drag(
                            ev.pointer_id(),
                            &pointer_drag_for_pointerleave,
                            set_dragging,
                            nodes_for_pointerleave.clone(),
                            edges_for_pointerleave.clone(),
                            set_positions,
                            dragging,
                            relax_generation_for_pointerleave.clone(),
                            false,
                            &ev,
                        );
                    } else {
                        graph_gesture_pointer_up(&graph_gesture_for_pointerleave, ev.pointer_id());
                    }
                }
                on:pointercancel=move |ev| {
                    if active_drag(&pointer_drag_for_pointercancel, ev.pointer_id()).is_some() {
                        finish_graph_drag(
                            ev.pointer_id(),
                            &pointer_drag_for_pointercancel,
                            set_dragging,
                            nodes_for_pointercancel.clone(),
                            edges_for_pointercancel.clone(),
                            set_positions,
                            dragging,
                            relax_generation_for_pointercancel.clone(),
                            false,
                            &ev,
                        );
                    } else {
                        graph_gesture_pointer_up(&graph_gesture_for_pointercancel, ev.pointer_id());
                    }
                }
            >
            <For
                each=move || graph_edges_for_edges.clone()
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
                each=move || { graph_nodes_for_nodes.clone() }
                key=|node| node.id
                children=move |node| {
                    let node_id = node.id;
                    let target = Page::Artist { id: node_id.to_string() };
                    let name = node.name;
                    let label = relation_node_label_text(&name);
                    let label_hit_width = relation_node_label_width(&name);
                    let relax_generation_for_node = relax_generation.clone();
                    let pointer_drag_for_node = pointer_drag.clone();
                    let pointer_drag_for_node_up = pointer_drag.clone();
                    let graph_gesture_for_node = graph_gesture.clone();
                    let last_tap_for_node = last_tap.clone();
                    let navigate = ctx.navigate;
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
                            on:pointerdown=move |ev| {
                                ev.prevent_default();
                                ev.stop_propagation();
                                capture_pointer(ev.as_ref(), ev.pointer_id());
                                if ev.pointer_type() == "touch" {
                                    if let Some(drag_start) = get_active_drag(&pointer_drag_for_node) {
                                        if drag_start.pointer_id != ev.pointer_id() {
                                            set_active_drag(&pointer_drag_for_node, None);
                                            set_dragging.set(None);
                                            let current_viewport = viewport.get_untracked();
                                            graph_gesture_pointer_down(
                                                &graph_gesture_for_node,
                                                drag_start.pointer_id,
                                                drag_start.client_x,
                                                drag_start.client_y,
                                                current_viewport,
                                            );
                                            graph_gesture_pointer_down(
                                                &graph_gesture_for_node,
                                                ev.pointer_id(),
                                                ev.client_x() as f64,
                                                ev.client_y() as f64,
                                                current_viewport,
                                            );
                                            return;
                                        }
                                    }

                                    if graph_gesture_is_active(&graph_gesture_for_node) {
                                        graph_gesture_pointer_down(
                                            &graph_gesture_for_node,
                                            ev.pointer_id(),
                                            ev.client_x() as f64,
                                            ev.client_y() as f64,
                                            viewport.get_untracked(),
                                        );
                                        return;
                                    }
                                }
                                relax_generation_for_node.fetch_add(1, Ordering::Relaxed);
                                set_dragging.set(Some(node_id));
                                set_active_drag(&pointer_drag_for_node, Some(GraphDragStart {
                                    id: node_id,
                                    pointer_id: ev.pointer_id(),
                                    client_x: ev.client_x() as f64,
                                    client_y: ev.client_y() as f64,
                                }));
                            }
                            on:pointerup=move |ev| {
                                ev.prevent_default();
                                let Some(drag_start) =
                                    active_drag(&pointer_drag_for_node_up, ev.pointer_id())
                                else {
                                    return;
                                };
                                if drag_start.id != node_id
                                    || pointer_distance(
                                        drag_start.client_x,
                                        drag_start.client_y,
                                        ev.client_x() as f64,
                                        ev.client_y() as f64,
                                    ) > TAP_MAX_MOVE_PX
                                {
                                    set_last_tap(&last_tap_for_node, None);
                                    return;
                                }
                                let now = js_sys::Date::now();
                                let client_x = ev.client_x() as f64;
                                let client_y = ev.client_y() as f64;
                                if is_double_tap(
                                    get_last_tap(&last_tap_for_node),
                                    node_id,
                                    now,
                                    client_x,
                                    client_y,
                                ) {
                                    set_last_tap(&last_tap_for_node, None);
                                    navigate.run(target.clone());
                                } else {
                                    set_last_tap(&last_tap_for_node, Some(GraphTap {
                                        id: node_id,
                                        time_ms: now,
                                        client_x,
                                        client_y,
                                    }));
                                }
                            }
                        >
                            <g transform=move || format!("scale({})", viewport.get().node_scale())>
                                <title>{name}</title>
                                <circle class="node-hit" r="26"></circle>
                                <rect
                                    class="node-label-hit"
                                    x="10"
                                    y="-16"
                                    width=label_hit_width.to_string()
                                    height="32"
                                    rx="6"
                                ></rect>
                                <circle class="node-dot" r="9"></circle>
                                <text x="14" y="4">{label}</text>
                            </g>
                        </g>
                    }
                }
            />
            </svg>
            <svg
                class="graph-minimap"
                viewBox=move || {
                    positions.with(|positions| {
                        layout_bounds_view_box(relation_graph_bounds_with_scale(
                            &graph_nodes_for_minimap,
                            positions,
                            GRAPH_FIT_PADDING,
                            viewport.get().node_scale(),
                        ))
                    })
                }
                aria-hidden="true"
            >
                <For
                    each=move || graph_edges_for_minimap.clone()
                    key=|edge| (edge.source, edge.target)
                    children=move |edge| {
                        let source = edge.source;
                        let target = edge.target;
                        view! {
                            <line
                                class="minimap-link"
                                x1=move || positions.with(|positions| layout_position(positions, source).0.to_string())
                                y1=move || positions.with(|positions| layout_position(positions, source).1.to_string())
                                x2=move || positions.with(|positions| layout_position(positions, target).0.to_string())
                                y2=move || positions.with(|positions| layout_position(positions, target).1.to_string())
                            />
                        }
                    }
                />
                <For
                    each=move || graph_nodes.clone()
                    key=|node| node.id
                    children=move |node| {
                        let node_id = node.id;
                        view! {
                            <circle
                                class="minimap-node"
                                cx=move || positions.with(|positions| layout_position(positions, node_id).0.to_string())
                                cy=move || positions.with(|positions| layout_position(positions, node_id).1.to_string())
                                r="5"
                            ></circle>
                        }
                    }
                />
                <rect
                    class="minimap-viewport"
                    x=move || viewport.get().x.to_string()
                    y=move || viewport.get().y.to_string()
                    width=move || viewport.get().width.to_string()
                    height=move || viewport.get().height.to_string()
                ></rect>
            </svg>
        </div>
    }
}

fn graph_pointer_position(
    ev: &leptos::ev::PointerEvent,
    viewport: GraphViewport,
) -> Option<(f64, f64)> {
    graph_event_position(
        ev.as_ref(),
        ev.client_x() as f64,
        ev.client_y() as f64,
        viewport,
    )
}

fn graph_wheel_position(
    ev: &leptos::ev::WheelEvent,
    viewport: GraphViewport,
) -> Option<(f64, f64)> {
    graph_event_position(
        ev.as_ref(),
        ev.client_x() as f64,
        ev.client_y() as f64,
        viewport,
    )
}

fn graph_event_position(
    event: &JsValue,
    client_x: f64,
    client_y: f64,
    viewport: GraphViewport,
) -> Option<(f64, f64)> {
    let metrics = graph_render_metrics(event, viewport)?;
    Some((
        viewport.x + (client_x - metrics.left - metrics.offset_x) / metrics.scale,
        viewport.y + (client_y - metrics.top - metrics.offset_y) / metrics.scale,
    ))
}

struct GraphRenderMetrics {
    left: f64,
    top: f64,
    scale: f64,
    offset_x: f64,
    offset_y: f64,
}

fn graph_render_metrics(event: &JsValue, viewport: GraphViewport) -> Option<GraphRenderMetrics> {
    let target = js_get!(event, "currentTarget").ok()?;
    if target.is_null() || target.is_undefined() {
        return None;
    }

    let rect_fn = js_function!(&target, "getBoundingClientRect").ok()?;
    let rect = rect_fn.call0(&target).ok()?;
    let left = js_property_number(&rect, "left")?;
    let top = js_property_number(&rect, "top")?;
    let width = js_property_number(&rect, "width")?.max(1.0);
    let height = js_property_number(&rect, "height")?.max(1.0);
    let scale = (width / viewport.width)
        .min(height / viewport.height)
        .max(0.001);
    let rendered_width = viewport.width * scale;
    let rendered_height = viewport.height * scale;
    Some(GraphRenderMetrics {
        left,
        top,
        scale,
        offset_x: (width - rendered_width) / 2.0,
        offset_y: (height - rendered_height) / 2.0,
    })
}

fn fit_graph_viewport(bounds: LayoutBounds) -> GraphViewport {
    let target_aspect = GRAPH_VIEWBOX_WIDTH / GRAPH_VIEWBOX_HEIGHT;
    let mut width = bounds.width().max(MIN_VIEWPORT_WIDTH);
    let mut height = bounds.height().max(MIN_VIEWPORT_WIDTH / target_aspect);
    let bounds_aspect = width / height;
    if bounds_aspect > target_aspect {
        height = width / target_aspect;
    } else {
        width = height * target_aspect;
    }

    GraphViewport {
        x: (bounds.min_x + bounds.max_x - width) / 2.0,
        y: (bounds.min_y + bounds.max_y - height) / 2.0,
        width,
        height,
    }
}

fn fit_graph_viewport_for_positions(
    nodes: &[RelationNode],
    positions: &[LayoutPosition],
) -> GraphViewport {
    let mut node_scale = 1.0;
    let mut viewport = fit_graph_viewport(relation_graph_bounds_with_scale(
        nodes,
        positions,
        GRAPH_FIT_PADDING,
        node_scale,
    ));

    for _ in 0..4 {
        node_scale = viewport.node_scale();
        viewport = fit_graph_viewport(relation_graph_bounds_with_scale(
            nodes,
            positions,
            GRAPH_FIT_PADDING,
            node_scale,
        ));
    }

    viewport
}

fn zoom_graph_viewport(viewport: GraphViewport, center: (f64, f64), factor: f64) -> GraphViewport {
    let min_width = MIN_VIEWPORT_WIDTH;
    let next_width = (viewport.width * factor).max(min_width);
    let next_height = next_width / (GRAPH_VIEWBOX_WIDTH / GRAPH_VIEWBOX_HEIGHT);
    let (center_x, center_y) = center;
    let rx = ((center_x - viewport.x) / viewport.width).clamp(0.0, 1.0);
    let ry = ((center_y - viewport.y) / viewport.height).clamp(0.0, 1.0);
    GraphViewport {
        x: center_x - next_width * rx,
        y: center_y - next_height * ry,
        width: next_width,
        height: next_height,
    }
}

fn layout_bounds_view_box(bounds: LayoutBounds) -> String {
    format!(
        "{} {} {} {}",
        bounds.min_x,
        bounds.min_y,
        bounds.width(),
        bounds.height()
    )
}

fn graph_gesture_pointer_down(
    gesture: &Arc<Mutex<GraphGesture>>,
    pointer_id: i32,
    client_x: f64,
    client_y: f64,
    viewport: GraphViewport,
) {
    if let Ok(mut gesture) = gesture.lock() {
        upsert_graph_pointer(
            &mut gesture.pointers,
            GraphPointer {
                pointer_id,
                client_x,
                client_y,
            },
        );
        if gesture.pointers.len() == 1 {
            gesture.pan = Some(GraphPanStart {
                pointer_id,
                client_x,
                client_y,
                viewport,
            });
            gesture.pinch = None;
        } else if gesture.pointers.len() >= 2 {
            gesture.pan = None;
            gesture.pinch = graph_pinch_start(&gesture.pointers, viewport);
        }
    }
}

fn graph_gesture_pointer_move(
    gesture: &Arc<Mutex<GraphGesture>>,
    event: &JsValue,
    pointer_id: i32,
    client_x: f64,
    client_y: f64,
) -> Option<GraphViewport> {
    let mut gesture = gesture.lock().ok()?;
    let pointer = GraphPointer {
        pointer_id,
        client_x,
        client_y,
    };
    upsert_graph_pointer(&mut gesture.pointers, pointer);

    if gesture.pointers.len() >= 2 {
        let pinch = gesture.pinch?;
        let (a, b) = (gesture.pointers[0], gesture.pointers[1]);
        let distance = pointer_distance(a.client_x, a.client_y, b.client_x, b.client_y).max(1.0);
        let factor = (pinch.distance / distance).clamp(0.25, 4.0);
        let mut next = zoom_graph_viewport(
            pinch.viewport,
            graph_event_position(event, pinch.center_x, pinch.center_y, pinch.viewport)?,
            factor,
        );
        next.x -= ((a.client_x + b.client_x) / 2.0 - pinch.center_x)
            / graph_render_metrics(event, next)?.scale;
        next.y -= ((a.client_y + b.client_y) / 2.0 - pinch.center_y)
            / graph_render_metrics(event, next)?.scale;
        return Some(next);
    }

    let pan = gesture.pan?;
    if pan.pointer_id != pointer_id {
        return None;
    }
    let metrics = graph_render_metrics(event, pan.viewport)?;
    Some(GraphViewport {
        x: pan.viewport.x - (client_x - pan.client_x) / metrics.scale,
        y: pan.viewport.y - (client_y - pan.client_y) / metrics.scale,
        width: pan.viewport.width,
        height: pan.viewport.height,
    })
}

fn graph_gesture_pointer_up(gesture: &Arc<Mutex<GraphGesture>>, pointer_id: i32) {
    if let Ok(mut gesture) = gesture.lock() {
        gesture
            .pointers
            .retain(|pointer| pointer.pointer_id != pointer_id);
        gesture.pan = None;
        gesture.pinch = None;
    }
}

fn graph_gesture_is_active(gesture: &Arc<Mutex<GraphGesture>>) -> bool {
    gesture
        .lock()
        .map(|gesture| !gesture.pointers.is_empty())
        .unwrap_or(false)
}

fn upsert_graph_pointer(pointers: &mut Vec<GraphPointer>, pointer: GraphPointer) {
    if let Some(existing) = pointers
        .iter_mut()
        .find(|existing| existing.pointer_id == pointer.pointer_id)
    {
        *existing = pointer;
    } else {
        pointers.push(pointer);
    }
}

fn graph_pinch_start(
    pointers: &[GraphPointer],
    viewport: GraphViewport,
) -> Option<GraphPinchStart> {
    let (a, b) = (*pointers.first()?, *pointers.get(1)?);
    Some(GraphPinchStart {
        distance: pointer_distance(a.client_x, a.client_y, b.client_x, b.client_y).max(1.0),
        center_x: (a.client_x + b.client_x) / 2.0,
        center_y: (a.client_y + b.client_y) / 2.0,
        viewport,
    })
}

fn active_drag(
    pointer_drag: &Arc<Mutex<Option<GraphDragStart>>>,
    pointer_id: i32,
) -> Option<GraphDragStart> {
    get_active_drag(pointer_drag).filter(|drag_start| drag_start.pointer_id == pointer_id)
}

fn get_active_drag(pointer_drag: &Arc<Mutex<Option<GraphDragStart>>>) -> Option<GraphDragStart> {
    pointer_drag.lock().ok().and_then(|drag_start| *drag_start)
}

fn set_active_drag(
    pointer_drag: &Arc<Mutex<Option<GraphDragStart>>>,
    value: Option<GraphDragStart>,
) {
    if let Ok(mut drag_start) = pointer_drag.lock() {
        *drag_start = value;
    }
}

fn get_last_tap(last_tap: &Arc<Mutex<Option<GraphTap>>>) -> Option<GraphTap> {
    last_tap.lock().ok().and_then(|tap| *tap)
}

fn set_last_tap(last_tap: &Arc<Mutex<Option<GraphTap>>>, value: Option<GraphTap>) {
    if let Ok(mut tap) = last_tap.lock() {
        *tap = value;
    }
}

fn finish_graph_drag(
    pointer_id: i32,
    pointer_drag: &Arc<Mutex<Option<GraphDragStart>>>,
    set_dragging: WriteSignal<Option<Id>>,
    nodes: Vec<RelationNode>,
    edges: Vec<RelationEdge>,
    set_positions: WriteSignal<Vec<LayoutPosition>>,
    dragging: ReadSignal<Option<Id>>,
    generation: Arc<AtomicU64>,
    prevent_default: bool,
    ev: &leptos::ev::PointerEvent,
) {
    if active_drag(pointer_drag, pointer_id).is_none() {
        return;
    }
    if prevent_default {
        ev.prevent_default();
    }
    set_active_drag(pointer_drag, None);
    set_dragging.set(None);
    start_layout_relax(nodes, edges, set_positions, dragging, generation);
}

fn capture_pointer(event: &JsValue, pointer_id: i32) {
    let Ok(target) = js_get!(event, "currentTarget") else {
        return;
    };
    if target.is_null() || target.is_undefined() {
        return;
    }
    let Ok(set_pointer_capture) = js_function!(&target, "setPointerCapture") else {
        return;
    };
    let _ = set_pointer_capture.call1(&target, &JsValue::from_f64(pointer_id as f64));
}

fn is_double_tap(
    previous: Option<GraphTap>,
    id: Id,
    time_ms: f64,
    client_x: f64,
    client_y: f64,
) -> bool {
    let Some(previous) = previous else {
        return false;
    };
    previous.id == id
        && time_ms - previous.time_ms <= DOUBLE_TAP_MAX_MS
        && pointer_distance(previous.client_x, previous.client_y, client_x, client_y)
            <= DOUBLE_TAP_MAX_DISTANCE_PX
}

fn pointer_distance(ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    let dx = ax - bx;
    let dy = ay - by;
    (dx * dx + dy * dy).sqrt()
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
    js_get!(value, key).ok()?.as_f64()
}
