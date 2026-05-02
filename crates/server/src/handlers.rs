use crate::application::artists as artists_app;
use crate::application::catalog::{
    self as catalog_app, CatalogEntityKind, ListAlbumsInput, ListArtistsInput, ListEventsInput,
    ListTracksInput,
};
use crate::application::lyrics as lyrics_app;
use crate::application::maintenance as maintenance_app;
use crate::application::relations as relations_app;
use crate::application::scan_jobs as scan_jobs_app;
use crate::application::settings as settings_app;
use crate::domain::{AlbumId, ArtistId, EntityId, EventId, ScanJobId, ScanJobState, TrackId};
use crate::services::{artwork as artwork_service, hls_cache, playback, scan, track_duration};
use crate::{ApiResult, AppError, AppState};
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::{Html, IntoResponse, Response};
use easy_musiclib_shared::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    cursor: Option<i64>,
    offset: Option<i64>,
    limit: Option<i64>,
    artist_id: Option<String>,
    album_id: Option<String>,
    event_id: Option<String>,
    liked: Option<bool>,
    q: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    q: String,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct RelationQuery {
    artist_id: Option<String>,
    scope: Option<String>,
    depth: Option<i64>,
    limit_nodes: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ArtworkQuery {
    size: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct StreamQuery {
    start_ms: Option<i64>,
    buffered: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct LyricsQuery {
    track_id: Option<String>,
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    duration_ms: Option<i64>,
}

pub async fn index(State(state): State<AppState>) -> impl IntoResponse {
    let path = state.static_dir.join("index.html");
    match tokio::fs::read_to_string(path).await {
        Ok(html) => Html(html).into_response(),
        Err(_) => Html(crate::app::FALLBACK_HTML).into_response(),
    }
}

pub async fn list_tracks(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<ListResponse<TrackSummary>>> {
    let artist_id = resolve_opt(&state, CatalogEntityKind::Artists, q.artist_id).await?;
    let album_id = resolve_opt(&state, CatalogEntityKind::Albums, q.album_id).await?;
    let event_id = resolve_opt(&state, CatalogEntityKind::Events, q.event_id).await?;
    Ok(Json(
        catalog_app::list_tracks(
            &state.repositories.catalog,
            ListTracksInput {
                cursor: q.cursor,
                offset: q.offset,
                limit: q.limit.unwrap_or(50),
                artist_id: artist_id.map(to_artist_id),
                album_id: album_id.map(to_album_id),
                event_id: event_id.map(to_event_id),
                liked: q.liked,
                q: q.q,
            },
        )
        .await?
        .into(),
    ))
}

pub async fn get_track(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<TrackDetail>> {
    let id =
        catalog_app::resolve_id(&state.repositories.catalog, CatalogEntityKind::Tracks, id).await?;
    Ok(Json(
        track_duration::fetch_track_detail_with_duration(&state, to_track_id(id)).await?,
    ))
}

pub async fn list_albums(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<ListResponse<AlbumSummary>>> {
    let artist_id = resolve_opt(&state, CatalogEntityKind::Artists, q.artist_id).await?;
    let event_id = resolve_opt(&state, CatalogEntityKind::Events, q.event_id).await?;
    Ok(Json(
        catalog_app::list_albums(
            &state.repositories.catalog,
            ListAlbumsInput {
                cursor: q.cursor,
                offset: q.offset,
                limit: q.limit.unwrap_or(50),
                artist_id: artist_id.map(to_artist_id),
                event_id: event_id.map(to_event_id),
                liked: q.liked,
                q: q.q,
            },
        )
        .await?
        .into(),
    ))
}

pub async fn get_album(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<AlbumDetail>> {
    let id =
        catalog_app::resolve_id(&state.repositories.catalog, CatalogEntityKind::Albums, id).await?;
    Ok(Json(
        catalog_app::fetch_album_detail(&state.repositories.catalog, to_album_id(id))
            .await?
            .into(),
    ))
}

pub async fn list_artists(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<ListResponse<ArtistSummary>>> {
    Ok(Json(
        catalog_app::list_artists(
            &state.repositories.catalog,
            ListArtistsInput {
                cursor: q.cursor,
                offset: q.offset,
                limit: q.limit.unwrap_or(50),
                liked: q.liked,
                q: q.q,
            },
        )
        .await?
        .into(),
    ))
}

pub async fn get_artist(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ArtistDetail>> {
    let id = catalog_app::resolve_id(&state.repositories.catalog, CatalogEntityKind::Artists, id)
        .await?;
    Ok(Json(
        catalog_app::fetch_artist_detail(&state.repositories.catalog, to_artist_id(id))
            .await?
            .into(),
    ))
}

pub async fn list_events(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<ListResponse<EventSummary>>> {
    Ok(Json(
        catalog_app::list_events(
            &state.repositories.catalog,
            ListEventsInput {
                cursor: q.cursor,
                offset: q.offset,
                limit: q.limit.unwrap_or(50),
                liked: q.liked,
                q: q.q,
            },
        )
        .await?
        .into(),
    ))
}

pub async fn get_event(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<EventDetail>> {
    let id =
        catalog_app::resolve_id(&state.repositories.catalog, CatalogEntityKind::Events, id).await?;
    Ok(Json(
        catalog_app::fetch_event_detail(&state.repositories.catalog, to_event_id(id))
            .await?
            .into(),
    ))
}

pub async fn search(
    State(state): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> ApiResult<Json<SearchResponse>> {
    Ok(Json(
        catalog_app::search(&state.repositories.catalog, q.q, q.limit.unwrap_or(50))
            .await?
            .into(),
    ))
}

pub async fn relations(
    State(state): State<AppState>,
    Query(q): Query<RelationQuery>,
) -> ApiResult<Json<RelationGraph>> {
    let artist_id = if q.scope.as_deref() == Some("all") {
        None
    } else {
        q.artist_id.as_deref()
    };
    Ok(Json(
        relations_app::relation_graph(
            &state.repositories.relations,
            artist_id,
            q.depth.unwrap_or(2),
            q.limit_nodes.unwrap_or(500),
        )
        .await?
        .into(),
    ))
}

pub async fn patch_track(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(patch): Json<LikePatch>,
) -> ApiResult<Json<TrackDetail>> {
    let id =
        catalog_app::resolve_id(&state.repositories.catalog, CatalogEntityKind::Tracks, id).await?;
    catalog_app::set_liked(
        &state.repositories.catalog,
        CatalogEntityKind::Tracks,
        id,
        patch.liked,
    )
    .await?;
    Ok(Json(
        track_duration::fetch_track_detail_with_duration(&state, to_track_id(id)).await?,
    ))
}

pub async fn patch_album(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(patch): Json<LikePatch>,
) -> ApiResult<Json<AlbumDetail>> {
    let id =
        catalog_app::resolve_id(&state.repositories.catalog, CatalogEntityKind::Albums, id).await?;
    catalog_app::set_liked(
        &state.repositories.catalog,
        CatalogEntityKind::Albums,
        id,
        patch.liked,
    )
    .await?;
    Ok(Json(
        catalog_app::fetch_album_detail(&state.repositories.catalog, to_album_id(id))
            .await?
            .into(),
    ))
}

pub async fn patch_artist(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(patch): Json<LikePatch>,
) -> ApiResult<Json<ArtistDetail>> {
    let id = catalog_app::resolve_id(&state.repositories.catalog, CatalogEntityKind::Artists, id)
        .await?;
    catalog_app::set_liked(
        &state.repositories.catalog,
        CatalogEntityKind::Artists,
        id,
        patch.liked,
    )
    .await?;
    Ok(Json(
        catalog_app::fetch_artist_detail(&state.repositories.catalog, to_artist_id(id))
            .await?
            .into(),
    ))
}

pub async fn patch_event(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(patch): Json<LikePatch>,
) -> ApiResult<Json<EventDetail>> {
    let id =
        catalog_app::resolve_id(&state.repositories.catalog, CatalogEntityKind::Events, id).await?;
    catalog_app::set_liked(
        &state.repositories.catalog,
        CatalogEntityKind::Events,
        id,
        patch.liked,
    )
    .await?;
    Ok(Json(
        catalog_app::fetch_event_detail(&state.repositories.catalog, to_event_id(id))
            .await?
            .into(),
    ))
}

pub async fn create_artist(
    State(state): State<AppState>,
    Json(req): Json<CreateArtistRequest>,
) -> ApiResult<Json<ArtistSummary>> {
    Ok(Json(
        artists_app::create_artist(&state.repositories.artists, &req.name)
            .await?
            .into(),
    ))
}

pub async fn add_alias(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<AddAliasRequest>,
) -> ApiResult<Json<MessageResponse>> {
    artists_app::add_artist_alias(&state.repositories.artists, &id, &req.alias).await?;
    Ok(Json(MessageResponse {
        message: "alias added".to_string(),
    }))
}

pub async fn merge_artists(
    State(state): State<AppState>,
    Json(req): Json<MergeArtistsRequest>,
) -> ApiResult<Json<MessageResponse>> {
    artists_app::merge_artists(
        &state.repositories.artists,
        &req.target,
        &req.source,
        req.by_name,
        "manual",
    )
    .await?;
    Ok(Json(MessageResponse {
        message: "artists merged".to_string(),
    }))
}

pub async fn auto_merge(State(state): State<AppState>) -> ApiResult<Json<MessageResponse>> {
    let count = artists_app::auto_merge(&state.repositories.artists).await?;
    Ok(Json(MessageResponse {
        message: format!("auto merge completed: {count}"),
    }))
}

pub async fn alias_csv_import(
    State(state): State<AppState>,
    Json(req): Json<AliasCsvImportRequest>,
) -> ApiResult<Json<MessageResponse>> {
    let count = artists_app::import_alias_csv(&state.repositories.artists, &req.csv).await?;
    Ok(Json(MessageResponse {
        message: format!("alias csv imported: {count} merges"),
    }))
}

pub async fn create_scan_job(
    State(state): State<AppState>,
    Json(req): Json<ScanJobRequest>,
) -> ApiResult<Json<ScanJobStatus>> {
    if req.roots.is_empty() {
        return Err(AppError::bad_request("roots is empty"));
    }
    let job =
        scan_jobs_app::insert_or_update_scan_job(&state.repositories.scan_jobs, &req.roots).await?;
    scan::spawn_scan(
        state.repositories.scan_jobs.clone(),
        state.repositories.scan_library.clone(),
        state.services.library_file_discovery.clone(),
        state.services.audio_metadata_reader.clone(),
        state.services.cue_sheet_reader.clone(),
        state.services.cue_renderer_selector.clone(),
        state.services.artist_name_parser.clone(),
        job.id,
        req.roots,
    );
    Ok(Json(job.into()))
}

pub async fn get_scan_job(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<Json<ScanJobStatus>> {
    Ok(Json(
        scan_jobs_app::scan_job(&state.repositories.scan_jobs, ScanJobId::new(id))
            .await?
            .into(),
    ))
}

pub async fn cancel_scan_job(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<Json<MessageResponse>> {
    // TODO: stop the scan job
    scan_jobs_app::update_scan_job_counts(
        &state.repositories.scan_jobs,
        ScanJobId::new(id),
        ScanJobState::CancelRequested,
        None,
        None,
        None,
        false,
    )
    .await?;
    Ok(Json(MessageResponse {
        message: "cancel requested".to_string(),
    }))
}

pub async fn vacuum(State(state): State<AppState>) -> ApiResult<Json<MessageResponse>> {
    maintenance_app::vacuum(&state.repositories.maintenance).await?;
    Ok(Json(MessageResponse {
        message: "database vacuum completed".to_string(),
    }))
}

pub async fn lyrics_search(
    State(state): State<AppState>,
    Query(q): Query<LyricsQuery>,
) -> ApiResult<Json<Vec<LyricsCandidate>>> {
    let input = if let Some(track_id) = q.track_id {
        let id = catalog_app::resolve_id(
            &state.repositories.catalog,
            CatalogEntityKind::Tracks,
            track_id,
        )
        .await?;
        let id = to_track_id(id);
        let detail = track_duration::fetch_track_detail_with_duration(&state, id).await?;
        lyrics_app::LyricsSearchInput {
            track_id: Some(id),
            title: detail.summary.title,
            artist: detail
                .summary
                .artists
                .iter()
                .map(|a| a.name.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            album: detail.summary.album.map(|a| a.name),
            duration_ms: detail.summary.duration_ms,
        }
    } else {
        lyrics_app::LyricsSearchInput {
            track_id: None,
            title: q
                .title
                .ok_or_else(|| AppError::bad_request("title is required"))?,
            artist: q.artist.unwrap_or_default(),
            album: q.album,
            duration_ms: q.duration_ms,
        }
    };

    Ok(Json(
        lyrics_app::search_lyrics(
            &state.repositories.lyrics_cache,
            &state.services.lyrics_provider,
            input,
        )
        .await?
        .into_iter()
        .map(Into::into)
        .collect(),
    ))
}

pub async fn get_settings(State(state): State<AppState>) -> ApiResult<Json<AppSettings>> {
    Ok(Json(
        settings_app::get_settings(&state.repositories.settings)
            .await?
            .into(),
    ))
}

pub async fn patch_settings(
    State(state): State<AppState>,
    Json(req): Json<UpdateAppSettingsRequest>,
) -> ApiResult<Json<AppSettings>> {
    Ok(Json(
        settings_app::update_settings(&state.repositories.settings, req.into())
            .await?
            .into(),
    ))
}

pub async fn clear_hls_cache() -> ApiResult<Json<HlsCacheClearResponse>> {
    Ok(Json(hls_cache::clear_hls_cache().await?))
}

pub async fn artwork(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(q): Query<ArtworkQuery>,
) -> ApiResult<Response> {
    artwork_service::artwork_response(&state, id, q.size.unwrap_or(256)).await
}

pub async fn stream_track(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<StreamQuery>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    playback::stream_track_response(
        state,
        id,
        q.start_ms.unwrap_or(0),
        q.buffered.unwrap_or(false),
        headers,
    )
    .await
}

pub async fn stream_track_hls_file(
    State(state): State<AppState>,
    Path((id, file)): Path<(String, String)>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    playback::stream_track_hls_file_response(state, id, file, headers).await
}

pub async fn download_track(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    playback::download_track_response(state, id, headers).await
}

async fn resolve_opt(
    state: &AppState,
    kind: CatalogEntityKind,
    value: Option<String>,
) -> ApiResult<Option<EntityId>> {
    match value {
        Some(value) if !value.trim().is_empty() => Ok(Some(
            catalog_app::resolve_id(&state.repositories.catalog, kind, value).await?,
        )),
        _ => Ok(None),
    }
}

fn to_track_id(id: EntityId) -> TrackId {
    TrackId::new(id.raw())
}

fn to_album_id(id: EntityId) -> AlbumId {
    AlbumId::new(id.raw())
}

fn to_artist_id(id: EntityId) -> ArtistId {
    ArtistId::new(id.raw())
}

fn to_event_id(id: EntityId) -> EventId {
    EventId::new(id.raw())
}
