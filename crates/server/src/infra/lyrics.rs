use crate::application::lyrics::LyricsProvider;
use crate::domain::LyricsCandidate;
use anyhow::Result;
use easy_musiclib_media::normalize::fuzzy_score;
use futures::FutureExt;
use futures::future::BoxFuture;
use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct NeteaseLyricsProvider;

impl LyricsProvider for NeteaseLyricsProvider {
    fn search_lyrics<'a>(
        &'a self,
        title: &'a str,
        artist: &'a str,
        album: Option<&'a str>,
        duration_ms: Option<i64>,
    ) -> BoxFuture<'a, Result<Vec<LyricsCandidate>>> {
        async move { search_netease(title, artist, album, duration_ms).await }.boxed()
    }
}

async fn search_netease(
    title: &str,
    artist: &str,
    album: Option<&str>,
    duration_ms: Option<i64>,
) -> Result<Vec<LyricsCandidate>> {
    let client = reqwest::Client::builder()
        .user_agent("easy-musiclib/0.1")
        .build()?;
    let album = album.unwrap_or_default();
    let keywords = [
        format!("{artist} - {album} - {title}"),
        format!("{album} - {title}"),
        format!("{artist} - {title}"),
        title.to_string(),
    ];

    let mut out = Vec::new();
    for keyword in keywords {
        let search: Value = client
            .get("https://music.163.com/api/search/get")
            .query(&[("s", keyword.as_str()), ("type", "1"), ("limit", "50")])
            .send()
            .await?
            .json()
            .await?;
        let Some(songs) = search
            .get("result")
            .and_then(|r| r.get("songs"))
            .and_then(|s| s.as_array())
        else {
            continue;
        };
        for song in songs.iter().take(8) {
            let song_duration = song.get("duration").and_then(|v| v.as_i64());
            if let (Some(a), Some(b)) = (song_duration, duration_ms) {
                if (a - b).abs() > 10_000 {
                    continue;
                }
            }
            let Some(song_id) = song.get("id").and_then(|v| v.as_i64()) else {
                continue;
            };
            let title2 = song
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let artist2 = song
                .get("artists")
                .and_then(|v| v.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|a| a.get("name").and_then(|v| v.as_str()))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            let album2 = song
                .get("album")
                .and_then(|a| a.get("name"))
                .and_then(|v| v.as_str())
                .map(ToOwned::to_owned);
            let (lrc, translated) = download_lyrics(&client, song_id).await?;
            if timestamp_count(&lrc) < 5 {
                continue;
            }
            let merged = merge_lyrics(&lrc, translated.as_deref().unwrap_or_default());
            let score = (fuzzy_score(title, &title2)
                + fuzzy_score(artist, &artist2)
                + fuzzy_score(album, album2.as_deref().unwrap_or_default()))
                / 3.0;
            out.push(LyricsCandidate {
                title: title2,
                artist: artist2,
                album: album2,
                duration_ms: song_duration,
                lyrics: merged,
                score,
                provider: "netease".to_string(),
            });
            if out.len() >= 9 {
                break;
            }
        }
        if out.len() >= 9 {
            break;
        }
    }
    out.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out.truncate(9);
    Ok(out)
}

async fn download_lyrics(
    client: &reqwest::Client,
    song_id: i64,
) -> Result<(String, Option<String>)> {
    let data: Value = client
        .get("https://music.163.com/api/song/lyric")
        .query(&[
            ("tv", "-1".to_string()),
            ("lv", "-1".to_string()),
            ("kv", "-1".to_string()),
            ("id", song_id.to_string()),
        ])
        .send()
        .await?
        .json()
        .await?;
    let lrc = data
        .get("lrc")
        .and_then(|v| v.get("lyric"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let translated = data
        .get("tlyric")
        .and_then(|v| v.get("lyric"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(ToOwned::to_owned);
    Ok((lrc, translated))
}

fn timestamp_count(lrc: &str) -> usize {
    lrc.lines()
        .filter(|line| line.starts_with('[') && line.get(3..4) == Some(":"))
        .count()
}

fn merge_lyrics(lrc: &str, translated: &str) -> String {
    let mut original = std::collections::BTreeMap::new();
    let mut trans = std::collections::BTreeMap::new();
    let mut unformatted = Vec::new();
    parse_lrc(lrc, &mut original, &mut unformatted);
    parse_lrc(translated, &mut trans, &mut Vec::new());
    let mut lines = unformatted;
    for stamp in original
        .keys()
        .chain(trans.keys())
        .collect::<std::collections::BTreeSet<_>>()
    {
        if let Some(line) = original.get(stamp) {
            lines.push(format!("{stamp}{line}"));
        }
        if let Some(line) = trans.get(stamp).filter(|line| !line.trim().is_empty()) {
            lines.push(format!("{stamp}{line}"));
        }
    }
    lines.join("\n")
}

fn parse_lrc(
    lrc: &str,
    out: &mut std::collections::BTreeMap<String, String>,
    unformatted: &mut Vec<String>,
) {
    for line in lrc.lines() {
        if line.len() >= 10 && line.starts_with('[') {
            if let Some(end) = line.find(']') {
                let stamp = &line[..=end];
                if stamp.len() >= 7 && stamp.as_bytes().get(3) == Some(&b':') {
                    out.insert(stamp.to_string(), line[end + 1..].to_string());
                    continue;
                }
            }
        }
        if !line.trim().is_empty() {
            unformatted.push(line.to_string());
        }
    }
}
