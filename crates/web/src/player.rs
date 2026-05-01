use crate::api::{api_get, api_patch_json};
use crate::app::{AppContext, PlayRequest};
use crate::lyrics::{
    LyricLine, active_lyric_index, apply_lyrics_text, load_lyrics_for_track, storage_key,
    storage_set,
};
use crate::media_session::{
    install_media_seek_handlers, js_error_text, update_media_position_state, update_media_session,
};
use crate::route::Page;
use crate::ui::{ArtistInlineLinks, EntityLink};
use crate::util::{format_time, progress_value};
use easy_musiclib_shared::{LikePatch, LyricsCandidate, TrackDetail, TrackSummary};
use leptos::prelude::*;
use std::rc::Rc;
use wasm_bindgen_futures::JsFuture;

#[component]
pub(crate) fn Player() -> impl IntoView {
    let ctx = expect_context::<AppContext>();
    let audio_ref = NodeRef::<leptos::html::Audio>::new();
    let (playing, set_playing) = signal(false);
    let (repeat, set_repeat) = signal(false);
    let (current_time, set_current_time) = signal(0.0_f64);
    let (duration, set_duration) = signal(0.0_f64);
    let (stream_start_time, set_stream_start_time) = signal(0.0_f64);
    let (seek_value, set_seek_value) = signal(0_i64);
    let (seeking, set_seeking) = signal(false);
    let (lyrics_open, set_lyrics_open) = signal(false);
    let (lyrics_selection_open, set_lyrics_selection_open) = signal(false);
    let (lyrics_lines, set_lyrics_lines) = signal(Vec::<LyricLine>::new());
    let (lyrics_text, set_lyrics_text) = signal(String::from("No track playing"));
    let (lyrics_loaded, set_lyrics_loaded) = signal(false);
    let (lyrics_candidates, set_lyrics_candidates) = signal(Vec::<LyricsCandidate>::new());
    let (active_line, set_active_line) = signal(-1_i64);

    let reset_lyrics = move |track: Option<TrackSummary>| {
        set_lyrics_lines.set(Vec::new());
        set_lyrics_candidates.set(Vec::new());
        set_lyrics_loaded.set(false);
        set_active_line.set(-1);
        set_lyrics_text.set(if track.is_some() {
            String::from("Loading...")
        } else {
            String::from("No track playing")
        });
    };

    let sync_lyrics = move || {
        let lines = lyrics_lines.get_untracked();
        if lines.is_empty() {
            return;
        }
        let now_ms = (current_time.get_untracked() * 1000.0 + 150.0) as i64;
        let index = active_lyric_index(&lines, now_ms);
        let previous = active_line.get_untracked();
        if previous == index {
            return;
        }
        set_active_line.set(index);
        if let Some(track) = ctx.current_track.get_untracked() {
            let is_playing = audio_ref
                .get()
                .map(|audio| !audio.paused())
                .unwrap_or(false);
            if !is_playing {
                return;
            }
            let title_override = index
                .try_into()
                .ok()
                .and_then(|index: usize| lines.get(index))
                .map(|line| line.text.as_str())
                .filter(|text| !text.is_empty());
            update_media_session(&track, title_override);
        }
    };

    let track_duration = move || {
        ctx.current_track
            .get_untracked()
            .and_then(|track| track.duration_ms)
            .map(|duration| duration.max(0) as f64 / 1000.0)
            .unwrap_or(0.0)
    };

    let clamp_position = move |position: f64| {
        let duration = track_duration();
        if duration > 0.0 {
            position.clamp(0.0, (duration - 0.05).max(0.0))
        } else {
            position.max(0.0)
        }
    };

    let set_display_position = move |position: f64| {
        let duration = track_duration();
        set_duration.set(duration);
        set_current_time.set(if duration > 0.0 {
            position.clamp(0.0, duration)
        } else {
            position.max(0.0)
        });
        if !seeking.get_untracked() {
            set_seek_value.set(progress_value(position, duration));
        }
    };

    let start_stream_at = Rc::new(move |position: f64, autoplay: bool| {
        let Some(track) = ctx.current_track.get_untracked() else {
            return;
        };
        let position = clamp_position(position);
        set_stream_start_time.set(position);
        set_display_position(position);
        if let Some(audio) = audio_ref.get() {
            let _ = audio.pause();
            let start_ms = (position * 1000.0).round().max(0.0) as i64;
            audio.set_src(&format!(
                "/api/tracks/{}/stream?start_ms={start_ms}",
                track.id
            ));
            audio.load();
            if autoplay {
                match audio.play() {
                    Ok(promise) => {
                        let set_status = ctx.set_status;
                        let title = track.title.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            match JsFuture::from(promise).await {
                                Ok(_) => set_status.set(format!("Playing {title}")),
                                Err(err) => set_status.set(js_error_text(err)),
                            }
                        });
                    }
                    Err(err) => ctx.set_status.set(js_error_text(err)),
                }
            }
        }
    });

    let media_seek_to = start_stream_at.clone();
    let media_seek_by = start_stream_at.clone();
    install_media_seek_handlers(
        move |position| media_seek_to(position, true),
        move |offset| media_seek_by(current_time.get_untracked() + offset, true),
    );

    let play_request_start_stream_at = start_stream_at.clone();
    Effect::new(move |_| {
        if let Some(request) = ctx.play_request.get() {
            let track = request.track;
            if !track.playable {
                ctx.set_status.set(String::from("Track is not playable"));
                return;
            }
            start_track_playback(
                track,
                ctx,
                play_request_start_stream_at.clone(),
                reset_lyrics,
                set_current_time,
                set_duration,
                set_stream_start_time,
                set_seek_value,
                set_lyrics_candidates,
                set_lyrics_lines,
                set_lyrics_text,
                set_lyrics_loaded,
            );
        }
    });

    let play_offset = move |offset: i64| {
        let list = ctx.playlist.get_untracked();
        if list.is_empty() {
            ctx.set_status.set(String::from("No playlist is active"));
            return;
        }
        let current = ctx.playlist_index.get_untracked();
        let next = (current + offset).rem_euclid(list.len() as i64);
        if let Some(track) = list.get(next as usize).cloned() {
            ctx.set_playlist_index.set(next);
            ctx.set_play_request.set(Some(PlayRequest { track }));
        }
    };

    let update_audio_progress = move || {
        if let Some(audio) = audio_ref.get() {
            let duration_value = track_duration();
            let current = if duration_value > 0.0 {
                (stream_start_time.get_untracked() + audio.current_time()).min(duration_value)
            } else {
                stream_start_time.get_untracked() + audio.current_time()
            };
            set_current_time.set(current);
            set_duration.set(duration_value);
            if !seeking.get_untracked() {
                set_seek_value.set(progress_value(current, duration_value));
            }
            update_media_position_state(current, duration_value, audio.playback_rate());
        }
        sync_lyrics();
    };

    let toggle_play = move |_| {
        let Some(audio) = audio_ref.get() else {
            return;
        };
        if ctx.current_track.get_untracked().is_none() {
            return;
        }
        if audio.paused() {
            match audio.play() {
                Ok(promise) => {
                    let set_status = ctx.set_status;
                    wasm_bindgen_futures::spawn_local(async move {
                        if let Err(err) = JsFuture::from(promise).await {
                            set_status.set(js_error_text(err));
                        }
                    });
                }
                Err(err) => ctx.set_status.set(js_error_text(err)),
            }
        } else {
            let _ = audio.pause();
        }
    };

    let toggle_like = move |_| {
        let Some(track) = ctx.current_track.get_untracked() else {
            return;
        };
        let liked = track.liked_at.is_none();
        wasm_bindgen_futures::spawn_local(async move {
            match api_patch_json::<TrackDetail, _>(
                &format!("/api/tracks/{}", track.id),
                &LikePatch { liked },
            )
            .await
            {
                Ok(updated) => {
                    let summary = updated.summary;
                    ctx.set_current_track.set(Some(summary.clone()));
                    ctx.set_track_update.set(Some(summary));
                }
                Err(err) => ctx.set_status.set(err),
            }
        });
    };

    let open_lyrics_selection = move |_| {
        set_lyrics_selection_open.set(true);
        if lyrics_candidates.get_untracked().is_empty() {
            if let Some(track) = ctx.current_track.get_untracked() {
                load_lyrics_for_track(
                    track,
                    true,
                    set_lyrics_candidates,
                    set_lyrics_lines,
                    set_lyrics_text,
                    set_lyrics_loaded,
                    ctx.set_status,
                );
            }
        }
    };

    let disable_lyrics = move |_| {
        if let Some(track) = ctx.current_track.get_untracked() {
            storage_set(&storage_key("lyrics_disabled", &track), "true");
            set_lyrics_lines.set(Vec::new());
            set_lyrics_loaded.set(false);
            set_lyrics_text.set(String::from("Disabled"));
            set_lyrics_selection_open.set(false);
        }
    };

    view! {
        <section class="player-bar" aria-label="Player">
            <button class="player-icon player-prev" type="button" aria-label="Previous track" on:click=move |_| play_offset(-1)>"⏮"</button>
            <button class="player-icon player-toggle" type="button" aria-label="Play or pause" on:click=toggle_play>
                {move || if playing.get() { "⏸" } else { "▶" }}
            </button>
            <button class="player-icon player-next" type="button" aria-label="Next track" on:click=move |_| play_offset(1)>"⏭"</button>

            <div class="player-track">
                <button class="player-art-button" type="button" aria-label="Open lyrics" on:click=move |_| set_lyrics_open.set(true)>
                    {move || ctx.current_track.get().and_then(|track| track.artwork_id).map(|id| view! {
                        <img src=format!("/api/artwork/{id}?size=160") alt="" />
                    })}
                </button>
                <div class="player-meta">
                    <strong>{move || ctx.current_track.get().map(|track| track.title).unwrap_or_else(|| String::from("No track playing"))}</strong>
                    <span>{move || ctx.current_track.get().map(|track| view! { <ArtistInlineLinks artists=track.artists /> })}</span>
                    <small>{move || ctx.current_track.get().and_then(|track| track.album).map(|album| view! { <EntityLink page=Page::Album { id: album.id.to_string() } label=album.name /> })}</small>
                </div>
            </div>

            <div class="player-progress">
                <span>{move || format_time(current_time.get())}</span>
                <input
                    type="range"
                    min="0"
                    max="1000"
                    step="1"
                    aria-label="Seek"
                    prop:value=seek_value
                    on:input=move |ev| {
                        set_seeking.set(true);
                        let value = event_target_value(&ev).parse::<i64>().unwrap_or(0);
                        set_seek_value.set(value);
                        let duration = duration.get_untracked();
                        if duration.is_finite() && duration > 0.0 {
                            set_current_time.set((value as f64 / 1000.0) * duration);
                        }
                    }
                    on:change={
                        let start_stream_at = start_stream_at.clone();
                        move |_| {
                            let duration = duration.get_untracked();
                            let target = if duration.is_finite() && duration > 0.0 {
                                (seek_value.get_untracked() as f64 / 1000.0) * duration
                            } else {
                                current_time.get_untracked()
                            };
                            let was_playing = audio_ref
                                .get()
                                .map(|audio| !audio.paused())
                                .unwrap_or(true);
                            set_seeking.set(false);
                            start_stream_at(target, was_playing);
                            update_audio_progress();
                        }
                    }
                />
                <span>{move || format_time(duration.get())}</span>
            </div>

            <div class="player-actions">
                <button
                    class=move || { if repeat.get() { "player-icon active" } else { "player-icon" } }
                    type="button"
                    aria-label="Repeat"
                    on:click=move |_| set_repeat.update(|value| *value = !*value)
                >
                    "↻"
                </button>
                <button class="player-icon" type="button" aria-label="Like" on:click=toggle_like>
                    {move || if ctx.current_track.get().and_then(|track| track.liked_at).is_some() { "♥" } else { "♡" }}
                </button>
                <a
                    class=move || { if ctx.current_track.get().is_some() { "player-icon" } else { "player-icon disabled" } }
                    href=move || ctx.current_track.get().map(|track| format!("/api/tracks/{}/download", track.id)).unwrap_or_else(|| String::from("#"))
                    aria-label="Download"
                    download
                >
                    "↓"
                </a>
            </div>
        </section>

        <audio
            node_ref=audio_ref
            preload="metadata"
            on:play=move |_| {
                set_playing.set(true);
                set_active_line.set(-2);
            }
            on:pause=move |_| {
                set_playing.set(false);
                if let Some(track) = ctx.current_track.get_untracked() {
                    update_media_session(&track, None);
                }
            }
            on:loadedmetadata=move |_| update_audio_progress()
            on:timeupdate=move |_| update_audio_progress()
            on:ended=move |_| {
                if repeat.get_untracked() {
                    start_stream_at(0.0, true);
                } else {
                    play_offset(1);
                }
            }
        ></audio>

        <section class="lyrics-popup" class:hidden=move || !lyrics_open.get() role="dialog" aria-modal="true">
            <button class="lyrics-close" type="button" aria-label="Close lyrics" on:click=move |_| set_lyrics_open.set(false)>"×"</button>
            <div class="lyrics-track">
                {move || ctx.current_track.get().and_then(|track| track.artwork_id).map(|id| view! {
                    <img src=format!("/api/artwork/{id}?size=512") alt="" />
                })}
                <strong>{move || ctx.current_track.get().map(|track| track.title).unwrap_or_else(|| String::from("No track playing"))}</strong>
                <span>{move || ctx.current_track.get().map(|track| view! { <ArtistInlineLinks artists=track.artists /> })}</span>
                <small>{move || ctx.current_track.get().and_then(|track| track.album).map(|album| view! { <EntityLink page=Page::Album { id: album.id.to_string() } label=album.name /> })}</small>
            </div>
            <div class="lyrics-panel">
                <div class="lyrics-header">
                    <h3>"Lyrics"</h3>
                    <button type="button" on:click=open_lyrics_selection>"Settings"</button>
                </div>
                <div class="lyrics-content">
                    {move || {
                        if lyrics_loaded.get() && !lyrics_lines.get().is_empty() {
                            view! {
                                <For
                                    each=move || { lyrics_lines.get().into_iter().enumerate().collect::<Vec<_>>() }
                                    key=|(index, line)| (*index, line.time_ms)
                                    children=move |(index, line)| view! {
                                        <div class=move || { if active_line.get() == index as i64 { "lyrics-line current" } else { "lyrics-line" } }>
                                            {line.text}
                                            <For
                                                each=move || line.translations.clone()
                                                key=|text| text.clone()
                                                children=move |text| view! { <span class="lyrics-translation">{text}</span> }
                                            />
                                        </div>
                                    }
                                />
                            }.into_any()
                        } else {
                            view! { <p>{lyrics_text.get()}</p> }.into_any()
                        }
                    }}
                </div>
            </div>
        </section>

        <section class="lyrics-selection-popup" class:hidden=move || !lyrics_selection_open.get() role="dialog" aria-modal="true">
            <div class="lyrics-selection-header">
                <h3>"Lyrics Settings"</h3>
                <div class="button-row">
                    <button type="button" on:click=disable_lyrics>"Disable Lyrics"</button>
                    <button type="button" aria-label="Close lyrics settings" on:click=move |_| set_lyrics_selection_open.set(false)>"×"</button>
                </div>
            </div>
            <div class="lyrics-options">
                {move || {
                    let candidates = lyrics_candidates.get();
                    if candidates.is_empty() {
                        view! { <p class="empty">"Lyrics not found"</p> }.into_any()
                    } else {
                        view! {
                            <For
                                each=move || lyrics_candidates.get()
                                key=|candidate| format!("{}:{}:{}", candidate.provider, candidate.title, candidate.score)
                                children=move |candidate| {
                                    let lyrics = candidate.lyrics.clone();
                                    view! {
                                        <button
                                            class="lyrics-option"
                                            type="button"
                                            on:click=move |_| {
                                                if let Some(track) = ctx.current_track.get_untracked() {
                                                    apply_lyrics_text(
                                                        &lyrics,
                                                        true,
                                                        Some(&track),
                                                        set_lyrics_lines,
                                                        set_lyrics_text,
                                                        set_lyrics_loaded,
                                                    );
                                                    storage_set(&storage_key("lyrics_disabled", &track), "false");
                                                }
                                                set_lyrics_selection_open.set(false);
                                            }
                                        >
                                            <strong>{candidate.title}</strong>
                                            <small>{[candidate.artist, candidate.album.unwrap_or_default(), candidate.provider].into_iter().filter(|item| !item.is_empty()).collect::<Vec<_>>().join(" / ")}</small>
                                            <pre>{candidate.lyrics.lines().take(12).collect::<Vec<_>>().join("\n")}</pre>
                                        </button>
                                    }
                                }
                            />
                        }.into_any()
                    }
                }}
            </div>
        </section>
    }
}

fn start_track_playback<F, R>(
    track: TrackSummary,
    ctx: AppContext,
    start_stream_at: Rc<F>,
    reset_lyrics: R,
    set_current_time: WriteSignal<f64>,
    set_duration: WriteSignal<f64>,
    set_stream_start_time: WriteSignal<f64>,
    set_seek_value: WriteSignal<i64>,
    set_lyrics_candidates: WriteSignal<Vec<LyricsCandidate>>,
    set_lyrics_lines: WriteSignal<Vec<LyricLine>>,
    set_lyrics_text: WriteSignal<String>,
    set_lyrics_loaded: WriteSignal<bool>,
) where
    F: Fn(f64, bool) + 'static,
    R: Fn(Option<TrackSummary>) + Copy + 'static,
{
    let begin = move |track: TrackSummary| {
        ctx.set_current_track.set(Some(track.clone()));
        ctx.set_track_update.set(Some(track.clone()));
        update_media_session(&track, None);
        reset_lyrics(Some(track.clone()));
        set_current_time.set(0.0);
        set_duration.set(
            track
                .duration_ms
                .map(|duration| duration.max(0) as f64 / 1000.0)
                .unwrap_or(0.0),
        );
        set_stream_start_time.set(0.0);
        set_seek_value.set(0);
        start_stream_at(0.0, true);
        load_lyrics_for_track(
            track,
            false,
            set_lyrics_candidates,
            set_lyrics_lines,
            set_lyrics_text,
            set_lyrics_loaded,
            ctx.set_status,
        );
    };

    if track.duration_ms.is_some() {
        begin(track);
        return;
    }

    wasm_bindgen_futures::spawn_local(async move {
        let track = match api_get::<TrackDetail>(&format!("/api/tracks/{}", track.id)).await {
            Ok(detail) => detail.summary,
            Err(err) => {
                ctx.set_status.set(err);
                track
            }
        };
        begin(track);
    });
}
