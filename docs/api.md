# API

The API uses JSON unless the route returns media bytes.

If accounts exist, API and media routes require a valid session cookie over
HTTPS. With zero accounts, the API remains open.

## Auth

```text
GET  /api/auth/status
POST /api/auth/login  { "username": "alice", "password": "..." }
POST /api/auth/logout
```

`login` sets an `HttpOnly`, `Secure`, `SameSite=Strict` session cookie.
Credentials are only accepted over HTTPS.

## Lists and Details

```text
GET /api/tracks?cursor=&offset=&limit=&artist_id=&album_id=&event_id=&liked=&q=
GET /api/tracks/{id_or_uuid}
GET /api/albums?cursor=&offset=&limit=&artist_id=&event_id=&liked=&q=
GET /api/albums/{id_or_uuid}
GET /api/artists?cursor=&offset=&limit=&liked=&q=
GET /api/artists/{id_or_uuid}
GET /api/events?cursor=&offset=&limit=&liked=&q=
GET /api/events/{id_or_uuid}
```

`limit` is capped server-side. IDs may be internal numeric IDs or UUIDs. List endpoints return:

```json
{
  "items": [],
  "next_cursor": null,
  "total": null
}
```

`total` is only present for responses that compute it.

## Search

```text
GET /api/search?q=&limit=50
```

Returns:

```json
{
  "tracks": [],
  "albums": [],
  "artists": [],
  "events": []
}
```

## Likes

```text
PATCH /api/tracks/{id_or_uuid}   { "liked": true }
PATCH /api/albums/{id_or_uuid}   { "liked": true }
PATCH /api/artists/{id_or_uuid}  { "liked": true }
PATCH /api/events/{id_or_uuid}   { "liked": true }
```

Set `liked` to `false` to unlike.

## Media

```text
GET /api/artwork/{artwork_id}?size=256
GET /api/tracks/{id_or_uuid}/raw
GET /api/tracks/{id_or_uuid}/stream?start_ms=0
GET /api/tracks/{id_or_uuid}/stream?start_ms=0&buffered=true
GET /api/tracks/{id_or_uuid}/hls/playlist.m3u8
GET /api/tracks/{id_or_uuid}/hls/init.mp4
GET /api/tracks/{id_or_uuid}/hls/segment_00000.m4s
GET /api/tracks/{id_or_uuid}/download
```

`raw` returns the original playable media entity when the server can expose one with a stable byte length. Ordinary files are served directly with HTTP Range. WAV and FLAC CUE tracks are rendered as lossless single-track cache files and then served with HTTP Range. Other CUE tracks do not expose raw playback.

`stream` returns browser-playback audio in the configured playback format. `start_ms` is optional and defaults to `0`; when present, the stream starts at that millisecond offset relative to the track. For CUE tracks, the offset is relative to the CUE track start and is clamped to the CUE track's duration. Streaming is sequential by default and returns `Accept-Ranges: none`; clients should restart the stream with a new `start_ms` to seek. Add `buffered=true` when the client needs a `Content-Length`/HTTP Range compatible media response, such as Safari or iOS WebView. If browser playback is set to `raw`, `stream` uses `raw_fallback` for transcoded playback.

`hls/{file}` returns generated FLAC fMP4 HLS files for playable tracks at the configured FLAC sample rate. Valid file names are `playlist.m3u8`, `init.mp4`, and segment files matching `segment_00000.m4s`. Requesting the playlist starts HLS generation if the cache is missing; files may briefly return `404` with `HLS file is not ready` while generation is still in progress. The playlist is returned as `application/vnd.apple.mpegurl`; init and segment files are returned as `audio/mp4` and support HTTP Range.

`download` returns the original/native track output. Ordinary file downloads support HTTP Range. CUE tracks are rendered as independent audio responses and do not expose the whole-album source file as a frontend URL.

## Settings

```text
GET   /api/settings
PATCH /api/settings { "browser_playback": { "format": "raw", "raw_fallback": "opus", "opus_bitrate": 256000, "flac_sample_rate": 48000 } }
PATCH /api/settings { "browser_playback": { "format": "raw", "raw_fallback": "flac", "opus_bitrate": 256000, "flac_sample_rate": 48000 } }
PATCH /api/settings { "browser_playback": { "format": "opus", "raw_fallback": "opus", "opus_bitrate": 256000, "flac_sample_rate": 48000 } }
PATCH /api/settings { "browser_playback": { "format": "flac", "raw_fallback": "opus", "opus_bitrate": 256000, "flac_sample_rate": 48000 } }
GET   /api/settings/accounts
POST  /api/settings/accounts { "username": "alice", "password": "..." }
PATCH /api/settings/accounts/{username} { "password": "..." }
DELETE /api/settings/accounts/{username}
```

Response:

```json
{
  "browser_playback": {
    "format": "opus",
    "raw_fallback": "opus",
    "opus_bitrate": 256000,
    "flac_sample_rate": 48000
  }
}
```

`browser_playback.format` controls the browser player's preferred playback mode. Supported values are `raw`, `opus`, and `flac`. Raw mode tries `/api/tracks/{id_or_uuid}/raw` first and falls back to `raw_fallback` when raw playback is unavailable. `raw_fallback` is either `opus` or `flac`. `opus_bitrate` is selected from common bitrates in bits per second: `64000`, `96000`, `128000`, `160000`, `192000`, `256000`, `320000`. `flac_sample_rate` is selected from common sample rates in Hz: `44100`, `48000`, `88200`, `96000`, `176400`, `192000`.

## Lyrics

```text
GET /api/lyrics/search?track_id=
GET /api/lyrics/search?title=&artist=&album=&duration_ms=
```

The default provider is NetEase. Results are cached in `lyric_cache`.

## Relations

```text
GET /api/relations?artist_id=&depth=2&limit_nodes=500
GET /api/relations?scope=all&limit_nodes=500
```

Response:

```json
{
  "nodes": [{"id": 1, "uuid": "...", "name": "..."}],
  "edges": [{"source": 1, "target": 2, "strength": 3, "details": []}]
}
```

## Scan Jobs

```text
POST /api/scan-jobs        { "roots": ["/music"] }
GET  /api/scan-jobs/{id}
POST /api/scan-jobs/{id}/cancel
```

`roots` must contain at least one path. Scan status responses include `id`, `status`, `root_paths`, file counters, timestamps, and an optional `error`.

## Cache

```text
POST /api/cache/hls/clear
```

Clears generated HLS cache files that are not currently being generated.

Response:

```json
{
  "cache_dir": "/tmp/easy_musiclib_hls",
  "removed_files": 0,
  "removed_dirs": 0,
  "removed_bytes": 0,
  "skipped_active_generators": 0
}
```


## Management

```text
POST /api/artists                  { "name": "Artist" }
POST /api/artists/{id}/aliases     { "alias": "Alias" }
POST /api/artists/merge            { "target": "1", "source": "2", "by_name": false }
POST /api/artists/auto-merge
POST /api/artists/alias-csv-import { "csv": "primary,alias1,alias2\n" }
POST /api/database/vacuum
```
