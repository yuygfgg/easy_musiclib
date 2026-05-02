use super::{entity_ref, now_ms};
use anyhow::Result;
use easy_musiclib_shared::{RelationEdge, RelationGraph, RelationNode};
use sqlx::{Row, SqlitePool};
use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};

pub async fn rebuild_relations(pool: &SqlitePool) -> Result<()> {
    sqlx::query("DELETE FROM artist_relation_edges")
        .execute(pool)
        .await?;
    let mut edges: BTreeMap<(i64, i64), (i64, BTreeSet<String>)> = BTreeMap::new();

    let rows = sqlx::query(
        "SELECT t.id AS track_id, t.uuid AS track_uuid, t.title, a.id AS artist_id, a.name
         FROM tracks t
         JOIN track_artists ta ON ta.track_id = t.id
         JOIN artists a ON a.id = ta.artist_id
         WHERE lower(a.name) <> 'various artists'
         ORDER BY t.id, ta.position",
    )
    .fetch_all(pool)
    .await?;
    let mut by_track: BTreeMap<i64, (String, String, Vec<i64>)> = BTreeMap::new();
    for row in rows {
        let track_id: i64 = row.try_get("track_id")?;
        by_track
            .entry(track_id)
            .or_insert_with(|| {
                (
                    row.try_get::<String, _>("title").unwrap_or_default(),
                    row.try_get::<String, _>("track_uuid").unwrap_or_default(),
                    Vec::new(),
                )
            })
            .2
            .push(row.try_get("artist_id")?);
    }
    for (_, (title, uuid, artists)) in by_track {
        add_pairs(&mut edges, &artists, format!("same song: {title} ({uuid})"));
    }

    let rows = sqlx::query(
        "SELECT al.id AS album_id, al.uuid AS album_uuid, al.title, a.id AS artist_id
         FROM albums al
         JOIN album_artists aa ON aa.album_id = al.id
         JOIN artists a ON a.id = aa.artist_id
         WHERE lower(a.name) <> 'various artists'
         ORDER BY al.id, aa.position",
    )
    .fetch_all(pool)
    .await?;
    let mut by_album: BTreeMap<i64, (String, String, Vec<i64>)> = BTreeMap::new();
    for row in rows {
        let album_id: i64 = row.try_get("album_id")?;
        by_album
            .entry(album_id)
            .or_insert_with(|| {
                (
                    row.try_get::<String, _>("title").unwrap_or_default(),
                    row.try_get::<String, _>("album_uuid").unwrap_or_default(),
                    Vec::new(),
                )
            })
            .2
            .push(row.try_get("artist_id")?);
    }
    for (_, (title, uuid, artists)) in &by_album {
        add_pairs(&mut edges, artists, format!("same album: {title} ({uuid})"));
    }

    let rows = sqlx::query(
        "SELECT DISTINCT al.id AS album_id, al.uuid AS album_uuid, al.title,
                aa.artist_id AS album_artist_id, ta.artist_id AS song_artist_id
         FROM albums al
         JOIN album_artists aa ON aa.album_id = al.id
         JOIN tracks t ON t.album_id = al.id
         JOIN track_artists ta ON ta.track_id = t.id
         WHERE aa.artist_id <> ta.artist_id",
    )
    .fetch_all(pool)
    .await?;
    for row in rows {
        let a: i64 = row.try_get("album_artist_id")?;
        let b: i64 = row.try_get("song_artist_id")?;
        let title: String = row.try_get("title")?;
        let uuid: String = row.try_get("album_uuid")?;
        add_edge(
            &mut edges,
            a,
            b,
            format!("album artist with song artist: {title} ({uuid})"),
        );
    }

    for ((a, b), (strength, details)) in edges {
        sqlx::query(
            "INSERT INTO artist_relation_edges
             (artist_a_id, artist_b_id, strength, details_json, updated_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(a)
        .bind(b)
        .bind(strength)
        .bind(serde_json::to_string(
            &details.into_iter().collect::<Vec<_>>(),
        )?)
        .bind(now_ms())
        .execute(pool)
        .await?;
    }
    Ok(())
}

fn add_pairs(
    edges: &mut BTreeMap<(i64, i64), (i64, BTreeSet<String>)>,
    artists: &[i64],
    detail: String,
) {
    for i in 0..artists.len() {
        for j in i + 1..artists.len() {
            add_edge(edges, artists[i], artists[j], detail.clone());
        }
    }
}

fn add_edge(
    edges: &mut BTreeMap<(i64, i64), (i64, BTreeSet<String>)>,
    a: i64,
    b: i64,
    detail: String,
) {
    if a == b {
        return;
    }
    let key = if a < b { (a, b) } else { (b, a) };
    let entry = edges.entry(key).or_insert_with(|| (0, BTreeSet::new()));
    entry.0 += 1;
    entry.1.insert(detail);
}

pub async fn relation_graph(
    pool: &SqlitePool,
    artist_id: Option<i64>,
    depth: i64,
    limit_nodes: i64,
) -> Result<RelationGraph> {
    let limit_nodes = limit_nodes.clamp(1, 2000) as usize;
    let mut node_ids = BTreeSet::new();
    let mut edge_keys = BTreeSet::new();

    if let Some(start) = artist_id {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::from([(start, 0i64)]);
        visited.insert(start);
        while let Some((id, d)) = queue.pop_front() {
            if node_ids.len() >= limit_nodes {
                break;
            }
            node_ids.insert(id);
            if d >= depth {
                continue;
            }
            let rows = sqlx::query(
                "SELECT artist_a_id, artist_b_id
                 FROM artist_relation_edges
                 WHERE artist_a_id = ? OR artist_b_id = ?",
            )
            .bind(id)
            .bind(id)
            .fetch_all(pool)
            .await?;
            for row in rows {
                let a: i64 = row.try_get("artist_a_id")?;
                let b: i64 = row.try_get("artist_b_id")?;
                edge_keys.insert((a.min(b), a.max(b)));
                let next = if a == id { b } else { a };
                if visited.insert(next) {
                    queue.push_back((next, d + 1));
                }
            }
        }
    } else {
        let rows = sqlx::query(
            "SELECT artist_a_id, artist_b_id
             FROM artist_relation_edges
             ORDER BY strength DESC
             LIMIT ?",
        )
        .bind(limit_nodes as i64)
        .fetch_all(pool)
        .await?;
        for row in rows {
            let a: i64 = row.try_get("artist_a_id")?;
            let b: i64 = row.try_get("artist_b_id")?;
            node_ids.insert(a);
            node_ids.insert(b);
            edge_keys.insert((a.min(b), a.max(b)));
            if node_ids.len() >= limit_nodes {
                break;
            }
        }
    }

    let mut nodes = Vec::new();
    for id in &node_ids {
        if let Ok(r) = entity_ref(pool, "artists", *id).await {
            nodes.push(RelationNode {
                id: r.id,
                uuid: r.uuid,
                name: r.name,
            });
        }
    }

    let mut edges = Vec::new();
    for (a, b) in edge_keys {
        if !node_ids.contains(&a) || !node_ids.contains(&b) {
            continue;
        }
        if let Some(row) = sqlx::query(
            "SELECT artist_a_id, artist_b_id, strength, details_json
             FROM artist_relation_edges WHERE artist_a_id = ? AND artist_b_id = ?",
        )
        .bind(a)
        .bind(b)
        .fetch_optional(pool)
        .await?
        {
            let details_json: String = row.try_get("details_json")?;
            edges.push(RelationEdge {
                source: row.try_get("artist_a_id")?,
                target: row.try_get("artist_b_id")?,
                strength: row.try_get("strength")?,
                details: serde_json::from_str(&details_json).unwrap_or_default(),
            });
        }
    }
    Ok(RelationGraph { nodes, edges })
}
