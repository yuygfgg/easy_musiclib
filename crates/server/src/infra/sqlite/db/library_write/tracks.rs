use anyhow::Result;
use easy_musiclib_media::normalize::normalize_name;
use sqlx::SqlitePool;
use uuid::Uuid;

pub async fn insert_track(
    pool: &SqlitePool,
    new_track: NewTrack<'_>,
    artist_ids: &[i64],
) -> Result<i64> {
    let res = sqlx::query(
        "INSERT INTO tracks
         (uuid, title, title_norm, album_id, event_id, cue_track_no, disc_no, track_no,
          duration_ms, date, year, artwork_id)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(new_track.title)
    .bind(normalize_name(new_track.title, false))
    .bind(new_track.album_id)
    .bind(new_track.event_id)
    .bind(new_track.cue_track_no)
    .bind(new_track.disc_no)
    .bind(new_track.track_no)
    .bind(new_track.duration_ms)
    .bind(new_track.date)
    .bind(new_track.year)
    .bind(new_track.artwork_id)
    .execute(pool)
    .await?;
    let track_id = res.last_insert_rowid();
    for (pos, artist_id) in artist_ids.iter().enumerate() {
        sqlx::query(
            "INSERT OR IGNORE INTO track_artists (track_id, artist_id, position) VALUES (?, ?, ?)",
        )
        .bind(track_id)
        .bind(artist_id)
        .bind(pos as i64)
        .execute(pool)
        .await?;
    }
    Ok(track_id)
}

pub struct NewTrack<'a> {
    pub title: &'a str,
    pub album_id: Option<i64>,
    pub event_id: Option<i64>,
    pub cue_track_no: Option<i64>,
    pub disc_no: Option<i64>,
    pub track_no: Option<i64>,
    pub duration_ms: Option<i64>,
    pub date: Option<&'a str>,
    pub year: Option<i64>,
    pub artwork_id: Option<i64>,
}

pub async fn insert_track_audio_source(
    pool: &SqlitePool,
    track_id: i64,
    src: NewTrackAudioSource<'_>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO track_audio_sources
         (track_id, kind, media_file_id, cue_sheet_id, codec, sample_rate,
          start_sample, end_sample, start_ms, end_ms, renderer)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(track_id)
    .bind(src.kind)
    .bind(src.media_file_id)
    .bind(src.cue_sheet_id)
    .bind(src.codec)
    .bind(src.sample_rate)
    .bind(src.start_sample)
    .bind(src.end_sample)
    .bind(src.start_ms)
    .bind(src.end_ms)
    .bind(src.renderer)
    .execute(pool)
    .await?;
    Ok(())
}

pub struct NewTrackAudioSource<'a> {
    pub kind: &'a str,
    pub media_file_id: i64,
    pub cue_sheet_id: Option<i64>,
    pub codec: &'a str,
    pub sample_rate: Option<i64>,
    pub start_sample: Option<i64>,
    pub end_sample: Option<i64>,
    pub start_ms: Option<i64>,
    pub end_ms: Option<i64>,
    pub renderer: &'a str,
}
