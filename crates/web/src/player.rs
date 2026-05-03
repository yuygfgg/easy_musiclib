use crate::api::{api_get, api_patch_json, spawn_playback_settings_load};
use crate::app::{AppContext, PlayRequest};
use crate::hls_prefetch::{audio_supports_flac_hls, hls_url, prefetch_hls_playlist_tracks};
use crate::lyrics::{
    LyricLine, active_lyric_index, apply_lyrics_text, load_lyrics_for_track, storage_key,
    storage_set,
};
use crate::media_session::{
    install_media_seek_handlers, install_media_track_handlers, js_error_text,
    update_media_position_state, update_media_session,
};
use crate::route::Page;
use crate::ui::{ArtistInlineLinks, EntityLink};
use crate::util::{format_time, progress_value};
use easy_musiclib_macros::{js_get, match_any_view, spawn_async, spawn_result};
use easy_musiclib_shared::{
    BrowserPlaybackFormat, BrowserPlaybackSettings, BrowserRawFallbackFormat, LikePatch,
    LyricsCandidate, TrackDetail, TrackSummary,
};
use leptos::prelude::*;
use std::rc::Rc;
use wasm_bindgen::{JsCast, JsValue, prelude::Closure};
use wasm_bindgen_futures::JsFuture;

const HLS_START_DRIFT_TOLERANCE_SECONDS: f64 = 1.0;

#[derive(Clone)]
struct PendingHlsStart {
    url: String,
    position: f64,
    autoplay: bool,
}

#[component]
pub(crate) fn Player() -> impl IntoView {
    let ctx = expect_context::<AppContext>();
    let audio_ref = NodeRef::<leptos::html::Audio>::new();
    let lyrics_content_ref = NodeRef::<leptos::html::Div>::new();
    let (playing, set_playing) = signal(false);
    let (repeat, set_repeat) = signal(false);
    let (current_time, set_current_time) = signal(0.0_f64);
    let (duration, set_duration) = signal(0.0_f64);
    let (stream_start_time, set_stream_start_time) = signal(0.0_f64);
    let (hls_playback, set_hls_playback) = signal(false);
    let (raw_playback, set_raw_playback) = signal(false);
    let (raw_failed_track, set_raw_failed_track) = signal(None::<i64>);
    let (pending_hls_start, set_pending_hls_start) = signal(None::<PendingHlsStart>);
    let (browser_playback, set_browser_playback) = signal(BrowserPlaybackSettings::default());
    let (playback_mode, set_playback_mode) = signal(String::from("Idle"));
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

    Effect::new(move |_| {
        spawn_playback_settings_load(set_browser_playback);
    });

    Effect::new(move |_| {
        let Some(audio) = audio_ref.get() else {
            return;
        };
        if !audio_supports_flac_hls(&audio, browser_playback.get()) {
            return;
        }
        if !playing.get() {
            return;
        }
        let tracks = ctx.playlist.get();
        prefetch_hls_playlist_tracks(
            &tracks,
            ctx.playlist_index.get(),
            ctx.current_track.get().map(|track| track.id),
        );
    });

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

    let scroll_lyrics_content_ref = lyrics_content_ref.clone();
    Effect::new(move |_| {
        let index = active_line.get();
        let line_count = lyrics_lines.get().len();
        if !lyrics_open.get() || !lyrics_loaded.get() || index < 0 || index as usize >= line_count {
            return;
        }
        let content_ref = scroll_lyrics_content_ref.clone();
        let callback = Closure::once_into_js(move || {
            scroll_lyric_line_into_view(content_ref, index);
        });
        if let Some(window) = web_sys::window() {
            let _ = window.request_animation_frame(callback.unchecked_ref::<js_sys::Function>());
        }
    });

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
        if let Some(audio) = audio_ref.get() {
            let playback = browser_playback.get_untracked();
            if playback.format == BrowserPlaybackFormat::Raw
                && raw_failed_track.get_untracked() != Some(track.id)
            {
                let url = raw_url(track.id);
                let same_src = raw_playback.get_untracked() && audio.current_src().ends_with(&url);
                set_hls_playback.set(false);
                set_raw_playback.set(true);
                set_playback_mode.set(String::from("raw"));
                set_stream_start_time.set(0.0);
                set_display_position(position);
                if same_src {
                    set_pending_hls_start.set(None);
                    audio.set_current_time(position);
                } else {
                    let _ = audio.pause();
                    audio.set_src(&url);
                    set_pending_hls_start.set(Some(PendingHlsStart {
                        url,
                        position,
                        autoplay,
                    }));
                    audio.load();
                    audio.set_current_time(position);
                }
                if autoplay {
                    match audio.play() {
                        Ok(promise) => {
                            let set_status = ctx.set_status;
                            let title = track.title.clone();
                            spawn_async! {
                                match JsFuture::from(promise).await {
                                    Ok(_) => set_status.set(format!("Playing {title}")),
                                    Err(err) => set_status.set(js_error_text(err)),
                                }
                            };
                        }
                        Err(err) => ctx.set_status.set(js_error_text(err)),
                    }
                }
                return;
            }

            let playback = raw_fallback_playback(playback);
            if audio_supports_flac_hls(&audio, playback) {
                let url = hls_url(track.id);
                let same_src = hls_playback.get_untracked() && audio.current_src().ends_with(&url);
                set_hls_playback.set(true);
                set_raw_playback.set(false);
                set_playback_mode.set(format!("FLAC HLS {}Hz", playback.flac_sample_rate));
                set_stream_start_time.set(0.0);
                set_display_position(position);
                if same_src {
                    set_pending_hls_start.set(None);
                    audio.set_current_time(position);
                } else {
                    let _ = audio.pause();
                    audio.set_src(&url);
                    set_pending_hls_start.set(Some(PendingHlsStart {
                        url,
                        position,
                        autoplay,
                    }));
                    audio.load();
                    audio.set_current_time(position);
                }
                if autoplay {
                    match audio.play() {
                        Ok(promise) => {
                            let set_status = ctx.set_status;
                            let title = track.title.clone();
                            spawn_async! {
                                match JsFuture::from(promise).await {
                                    Ok(_) => set_status.set(format!("Playing {title}")),
                                    Err(err) => set_status.set(js_error_text(err)),
                                }
                            };
                        }
                        Err(err) => ctx.set_status.set(js_error_text(err)),
                    }
                }
                return;
            }

            set_pending_hls_start.set(None);
            set_hls_playback.set(false);
            set_raw_playback.set(false);
            set_playback_mode.set(stream_playback_mode(
                playback,
                needs_buffered_audio_response(),
            ));
            set_stream_start_time.set(position);
            set_display_position(position);
            let _ = audio.pause();
            let start_ms = (position * 1000.0).round().max(0.0) as i64;
            audio.set_src(&stream_url(track.id, start_ms));
            audio.load();
            if autoplay {
                match audio.play() {
                    Ok(promise) => {
                        let set_status = ctx.set_status;
                        let title = track.title.clone();
                        spawn_async! {
                            match JsFuture::from(promise).await {
                                Ok(_) => set_status.set(format!("Playing {title}")),
                                Err(err) => set_status.set(js_error_text(err)),
                            }
                        };
                    }
                    Err(err) => ctx.set_status.set(js_error_text(err)),
                }
            }
        }
    });

    let apply_pending_hls_start = move |confirm_playback: bool| {
        let Some(pending) = pending_hls_start.get_untracked() else {
            return;
        };
        let Some(audio) = audio_ref.get() else {
            return;
        };
        let current_src = audio.current_src();
        if !current_src.is_empty() && !current_src.ends_with(&pending.url) {
            set_pending_hls_start.set(None);
            return;
        }

        let current = audio.current_time();
        if hls_start_needs_seek(current, pending.position) {
            audio.set_current_time(pending.position);
            return;
        }
        if audio.seeking() {
            return;
        }
        if !pending.autoplay || confirm_playback {
            set_pending_hls_start.set(None);
        }
    };

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
            set_raw_failed_track.set(None);
            set_raw_playback.set(false);
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
    install_media_track_handlers(move || play_offset(-1), move || play_offset(1));

    let update_audio_progress = move || {
        if let Some(audio) = audio_ref.get() {
            let duration_value = track_duration();
            let current = if hls_playback.get_untracked() || raw_playback.get_untracked() {
                audio.current_time()
            } else if duration_value > 0.0 {
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
            if let Some(mut pending) = pending_hls_start.get_untracked() {
                pending.autoplay = true;
                set_pending_hls_start.set(Some(pending));
            }
            match audio.play() {
                Ok(promise) => {
                    let set_status = ctx.set_status;
                    spawn_async! {
                        if let Err(err) = JsFuture::from(promise).await {
                            set_status.set(js_error_text(err));
                        }
                    };
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
        spawn_result! {
            api_patch_json::<TrackDetail, _>(&format!("/api/tracks/{}", track.id), &LikePatch { liked }),
            Ok(updated) => {
                let summary = updated.summary;
                ctx.set_current_track.set(Some(summary.clone()));
                ctx.set_track_update.set(Some(summary));
            },
            Err(err) => { ctx.set_status.set(err); },
        };
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
            set_active_line.set(-1);
            set_lyrics_text.set(String::from("Disabled"));
            set_lyrics_selection_open.set(false);
            update_media_session(&track, None);
        }
    };

    let raw_error_start_stream_at = start_stream_at.clone();
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
                    <small class="player-mode">{move || playback_mode.get()}</small>
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
            on:playing=move |_| apply_pending_hls_start(false)
            on:pause=move |_| {
                set_playing.set(false);
                if let Some(track) = ctx.current_track.get_untracked() {
                    update_media_session(&track, None);
                }
            }
            on:loadedmetadata=move |_| {
                apply_pending_hls_start(false);
                update_audio_progress();
            }
            on:canplay=move |_| apply_pending_hls_start(false)
            on:seeked=move |_| apply_pending_hls_start(false)
            on:timeupdate=move |_| {
                apply_pending_hls_start(true);
                update_audio_progress();
            }
            on:error=move |_| {
                if raw_playback.get_untracked() {
                    if let Some(track) = ctx.current_track.get_untracked() {
                        set_raw_failed_track.set(Some(track.id));
                        let target = current_time.get_untracked();
                        ctx.set_status.set(String::from("Raw playback unavailable; falling back to transcoded stream"));
                        raw_error_start_stream_at(target, true);
                        return;
                    }
                }
                if let Some(audio) = audio_ref.get() {
                    ctx.set_status.set(audio_error_text(&audio));
                } else {
                    ctx.set_status.set(String::from("Audio error"));
                }
            }
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
                <div class="lyrics-art">
                    {move || ctx.current_track.get().and_then(|track| track.artwork_id).map(|id| view! {
                        <img src=format!("/api/artwork/{id}?size=512") alt="" />
                    })}
                </div>
                <strong>{move || ctx.current_track.get().map(|track| track.title).unwrap_or_else(|| String::from("No track playing"))}</strong>
                <span>{move || ctx.current_track.get().map(|track| view! { <ArtistInlineLinks artists=track.artists /> })}</span>
                <small>{move || ctx.current_track.get().and_then(|track| track.album).map(|album| view! { <EntityLink page=Page::Album { id: album.id.to_string() } label=album.name /> })}</small>
            </div>
            <div class="lyrics-panel">
                <div class="lyrics-header">
                    <h3>"Lyrics"</h3>
                    <button type="button" on:click=open_lyrics_selection>"Settings"</button>
                </div>
                <div class="lyrics-content" node_ref=lyrics_content_ref>
                    {move || match_any_view!(lyrics_loaded.get() && !lyrics_lines.get().is_empty(), {
                        true => view! {
                                <For
                                    each=move || { lyrics_lines.get().into_iter().enumerate().collect::<Vec<_>>() }
                                    key=|(index, line)| (*index, line.time_ms)
                                    children=move |(index, line)| view! {
                                        <div
                                            id=format!("lyrics-line-{index}")
                                            class=move || { if active_line.get() == index as i64 { "lyrics-line current" } else { "lyrics-line" } }
                                        >
                                            {line.text}
                                            <For
                                                each=move || line.translations.clone()
                                                key=|text| text.clone()
                                                children=move |text| view! { <span class="lyrics-translation">{text}</span> }
                                            />
                                        </div>
                                    }
                                />
                            },
                        false => view! { <p>{lyrics_text.get()}</p> },
                    })}
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
                {move || match_any_view!(lyrics_candidates.get().is_empty(), {
                    true => view! { <p class="empty">"Lyrics not found"</p> },
                    false => {
                    let candidates = lyrics_candidates.get();
                        view! {
                            <For
                                each=move || candidates.clone()
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
                        }
                    },
                })}
            </div>
        </section>
    }
}

fn scroll_lyric_line_into_view(content_ref: NodeRef<leptos::html::Div>, index: i64) {
    let Some(content) = content_ref.get() else {
        return;
    };
    let content_height = content.client_height();
    if content_height <= 0 {
        return;
    }
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Some(line) = document.get_element_by_id(&format!("lyrics-line-{index}")) else {
        return;
    };
    let Ok(line) = line.dyn_into::<web_sys::HtmlElement>() else {
        return;
    };

    let target = line.offset_top() - ((content_height - line.client_height()) / 2);
    content.set_scroll_top(target.max(0));
}

fn stream_url(track_id: i64, start_ms: i64) -> String {
    let mut url = format!("/api/tracks/{track_id}/stream?start_ms={start_ms}");
    if needs_buffered_audio_response() {
        url.push_str("&buffered=true");
    }
    url
}

fn raw_url(track_id: i64) -> String {
    format!("/api/tracks/{track_id}/raw")
}

fn hls_start_needs_seek(current: f64, target: f64) -> bool {
    if !current.is_finite() || !target.is_finite() {
        return false;
    }
    current > target + HLS_START_DRIFT_TOLERANCE_SECONDS
        || current + HLS_START_DRIFT_TOLERANCE_SECONDS < target
}

fn stream_playback_mode(playback: BrowserPlaybackSettings, buffered: bool) -> String {
    let transport = if buffered { "buffered" } else { "direct" };
    match playback.format {
        BrowserPlaybackFormat::Raw => match playback.raw_fallback {
            BrowserRawFallbackFormat::Opus => {
                format!(
                    "{transport} fallback Opus {}k",
                    playback.opus_bitrate / 1000
                )
            }
            BrowserRawFallbackFormat::Flac => {
                format!("{transport} fallback FLAC {}Hz", playback.flac_sample_rate)
            }
        },
        BrowserPlaybackFormat::Opus => {
            format!("{transport} Opus {}k", playback.opus_bitrate / 1000)
        }
        BrowserPlaybackFormat::Flac => {
            format!("{transport} FLAC {}Hz", playback.flac_sample_rate)
        }
    }
}

fn raw_fallback_playback(mut playback: BrowserPlaybackSettings) -> BrowserPlaybackSettings {
    if playback.format == BrowserPlaybackFormat::Raw {
        playback.format = match playback.raw_fallback {
            BrowserRawFallbackFormat::Opus => BrowserPlaybackFormat::Opus,
            BrowserRawFallbackFormat::Flac => BrowserPlaybackFormat::Flac,
        };
    }
    playback
}

fn needs_buffered_audio_response() -> bool {
    let ua = navigator_string_property("userAgent");
    let vendor = navigator_string_property("vendor");
    if ua.is_empty() {
        return false;
    }

    let apple_webkit = ua.contains("AppleWebKit");
    let apple_vendor = vendor.contains("Apple");
    let ios = ua.contains("iPhone")
        || ua.contains("iPad")
        || ua.contains("iPod")
        || (ua.contains("Macintosh") && navigator_number_property("maxTouchPoints") > 1.0);
    let safari = apple_vendor
        && ua.contains("Safari")
        && !ua.contains("Chrome")
        && !ua.contains("Chromium")
        && !ua.contains("CriOS")
        && !ua.contains("FxiOS")
        && !ua.contains("Edg")
        && !ua.contains("OPR");

    safari || (ios && apple_vendor && apple_webkit)
}

fn navigator_string_property(name: &str) -> String {
    navigator_property(name)
        .and_then(|value| value.as_string())
        .unwrap_or_default()
}

fn navigator_number_property(name: &str) -> f64 {
    navigator_property(name)
        .and_then(|value| value.as_f64())
        .unwrap_or(0.0)
}

fn navigator_property(name: &str) -> Option<JsValue> {
    let window = web_sys::window()?;
    let navigator = js_get!(window.as_ref(), "navigator").ok()?;
    js_get!(&navigator, name).ok()
}

fn audio_error_text(audio: &web_sys::HtmlAudioElement) -> String {
    let error = js_get!(audio.as_ref(), "error")
        .ok()
        .filter(|value| !value.is_null() && !value.is_undefined());
    let code = error
        .as_ref()
        .and_then(|value| js_number_property(value, "code"))
        .map(|value| value as u16);
    let message = error
        .as_ref()
        .and_then(|value| js_string_property(value, "message"));

    let mut text = match code {
        Some(1) => String::from("Audio error: MEDIA_ERR_ABORTED (1)"),
        Some(2) => String::from("Audio error: MEDIA_ERR_NETWORK (2)"),
        Some(3) => String::from("Audio error: MEDIA_ERR_DECODE (3)"),
        Some(4) => String::from("Audio error: MEDIA_ERR_SRC_NOT_SUPPORTED (4)"),
        Some(code) => format!("Audio error: MediaError code {code}"),
        None => String::from("Audio error"),
    };
    if let Some(message) = message.filter(|value| !value.is_empty()) {
        text.push_str(": ");
        text.push_str(&message);
    }

    text.push_str(&format!(
        " [ready={}, network={}",
        ready_state_text(audio.ready_state()),
        network_state_text(audio.network_state())
    ));
    let src = audio.current_src();
    if !src.is_empty() {
        text.push_str(", src=");
        text.push_str(&src);
    }
    text.push(']');
    text
}

fn js_string_property(value: &JsValue, key: &str) -> Option<String> {
    js_get!(value, key).ok().and_then(|value| value.as_string())
}

fn js_number_property(value: &JsValue, key: &str) -> Option<f64> {
    js_get!(value, key)
        .ok()
        .and_then(|value| value.as_f64())
        .filter(|value| value.is_finite())
}

fn ready_state_text(value: u16) -> &'static str {
    match value {
        0 => "HAVE_NOTHING",
        1 => "HAVE_METADATA",
        2 => "HAVE_CURRENT_DATA",
        3 => "HAVE_FUTURE_DATA",
        4 => "HAVE_ENOUGH_DATA",
        _ => "UNKNOWN",
    }
}

fn network_state_text(value: u16) -> &'static str {
    match value {
        0 => "NETWORK_EMPTY",
        1 => "NETWORK_IDLE",
        2 => "NETWORK_LOADING",
        3 => "NETWORK_NO_SOURCE",
        _ => "UNKNOWN",
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

    spawn_async! {
        let track = match api_get::<TrackDetail>(&format!("/api/tracks/{}", track.id)).await {
            Ok(detail) => detail.summary,
            Err(err) => {
                ctx.set_status.set(err);
                track
            }
        };
        begin(track);
    };
}
