use super::*;
use anyhow::Result;
use easy_musiclib_media::normalize::normalize_name;
use sqlx::{Row, SqlitePool, sqlite::SqlitePoolOptions};

async fn test_pool() -> Result<SqlitePool> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;
    crate::infra::sqlite::schema::init_db(&pool).await?;
    Ok(pool)
}

async fn insert_album_row(
    pool: &SqlitePool,
    uuid: &str,
    date: Option<&str>,
    year: Option<i64>,
    event_id: Option<i64>,
) -> Result<i64> {
    let res = sqlx::query(
        "INSERT INTO albums (uuid, title, title_norm, date, year, event_id)
             VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(uuid)
    .bind(uuid)
    .bind(normalize_name(uuid, false))
    .bind(date)
    .bind(year)
    .bind(event_id)
    .execute(pool)
    .await?;
    Ok(res.last_insert_rowid())
}

#[tokio::test]
async fn ensure_event_ignores_unknown_event() -> Result<()> {
    let pool = test_pool().await?;

    let event_id =
        ensure_event(&pool, Some("Unknown Event"), Some("2024-04-28"), Some(2024)).await?;

    assert_eq!(event_id, None);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM events")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 0);
    Ok(())
}

#[tokio::test]
async fn find_or_create_album_keeps_existing_year_when_date_year_differs() -> Result<()> {
    let pool = test_pool().await?;
    let event_id = ensure_event(&pool, Some("M3"), Some("2024-04-28"), Some(2024))
        .await?
        .expect("known event should be inserted");
    insert_album_row(&pool, "Existing Album", None, Some(2023), None).await?;

    let album_id = find_or_create_album(
        &pool,
        "Existing Album",
        &[],
        None,
        Some("2024-04-28"),
        Some(event_id),
        None,
    )
    .await?;

    let row = sqlx::query("SELECT date, year FROM albums WHERE id = ?")
        .bind(album_id)
        .fetch_one(&pool)
        .await?;
    assert_eq!(row.try_get::<Option<String>, _>("date")?, None);
    assert_eq!(row.try_get::<Option<i64>, _>("year")?, Some(2023));
    Ok(())
}

#[tokio::test]
async fn repair_event_dates_refines_album_only_when_year_matches() -> Result<()> {
    let pool = test_pool().await?;
    let event_id = ensure_event(&pool, Some("M3"), Some("2024-04-28"), Some(2024))
        .await?
        .expect("known event should be inserted");
    insert_album_row(&pool, "match", Some("2024"), Some(2024), Some(event_id)).await?;
    insert_album_row(
        &pool,
        "mismatch-short",
        Some("2023"),
        Some(2023),
        Some(event_id),
    )
    .await?;
    insert_album_row(&pool, "mismatch-empty", None, Some(2023), Some(event_id)).await?;

    repair_event_dates_and_artwork(&pool).await?;

    let matched: Option<String> =
        sqlx::query_scalar("SELECT date FROM albums WHERE uuid = 'match'")
            .fetch_one(&pool)
            .await?;
    let mismatch_short: Option<String> =
        sqlx::query_scalar("SELECT date FROM albums WHERE uuid = 'mismatch-short'")
            .fetch_one(&pool)
            .await?;
    let mismatch_empty: Option<String> =
        sqlx::query_scalar("SELECT date FROM albums WHERE uuid = 'mismatch-empty'")
            .fetch_one(&pool)
            .await?;

    assert_eq!(matched.as_deref(), Some("2024-04-28"));
    assert_eq!(mismatch_short.as_deref(), Some("2023"));
    assert_eq!(mismatch_empty, None);
    Ok(())
}

#[tokio::test]
async fn repair_event_dates_ignores_existing_unknown_events() -> Result<()> {
    let pool = test_pool().await?;
    let event_id = sqlx::query(
        "INSERT INTO events (uuid, name, name_norm, date, year)
             VALUES ('event-unknown', 'Unknown Event', 'unknown event', '2024-04-28', 2024)",
    )
    .execute(&pool)
    .await?
    .last_insert_rowid();
    let album_id = insert_album_row(&pool, "unknown-linked", None, None, Some(event_id)).await?;

    repair_event_dates_and_artwork(&pool).await?;

    let row = sqlx::query("SELECT date, year FROM albums WHERE id = ?")
        .bind(album_id)
        .fetch_one(&pool)
        .await?;
    assert_eq!(row.try_get::<Option<String>, _>("date")?, None);
    assert_eq!(row.try_get::<Option<i64>, _>("year")?, None);
    Ok(())
}

#[tokio::test]
async fn discard_unknown_events_unlinks_and_removes_existing_rows() -> Result<()> {
    let pool = test_pool().await?;
    let event_id = sqlx::query(
        "INSERT INTO events (uuid, name, name_norm, date, year)
             VALUES ('event-unknown', 'Unknown Event', 'unknown event', '2024-04-28', 2024)",
    )
    .execute(&pool)
    .await?
    .last_insert_rowid();
    let album_id = insert_album_row(&pool, "unknown-linked", None, None, Some(event_id)).await?;
    let track_id = insert_track(
        &pool,
        NewTrack {
            title: "Unknown Event Track",
            album_id: Some(album_id),
            event_id: Some(event_id),
            cue_track_no: None,
            disc_no: None,
            track_no: None,
            duration_ms: None,
            date: None,
            year: None,
            artwork_id: None,
        },
        &[],
    )
    .await?;
    sqlx::query("INSERT INTO event_albums (event_id, album_id) VALUES (?, ?)")
        .bind(event_id)
        .bind(album_id)
        .execute(&pool)
        .await?;
    refresh_event_search(&pool, event_id).await?;

    discard_unknown_events(&pool).await?;

    let event_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM events")
        .fetch_one(&pool)
        .await?;
    let link_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_albums")
        .fetch_one(&pool)
        .await?;
    let event_search_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM search_index WHERE kind = 'event'")
            .fetch_one(&pool)
            .await?;
    let album_event_id: Option<i64> =
        sqlx::query_scalar("SELECT event_id FROM albums WHERE id = ?")
            .bind(album_id)
            .fetch_one(&pool)
            .await?;
    let track_event_id: Option<i64> =
        sqlx::query_scalar("SELECT event_id FROM tracks WHERE id = ?")
            .bind(track_id)
            .fetch_one(&pool)
            .await?;

    assert_eq!(event_count, 0);
    assert_eq!(link_count, 0);
    assert_eq!(event_search_count, 0);
    assert_eq!(album_event_id, None);
    assert_eq!(track_event_id, None);
    Ok(())
}
