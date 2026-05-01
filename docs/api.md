# API

The API uses JSON unless the route returns media bytes.

## Lists and Details

```text
GET /api/tracks?cursor=&limit=&artist_id=&album_id=&event_id=&liked=
GET /api/tracks/{id_or_uuid}
GET /api/albums?cursor=&limit=&artist_id=&event_id=&liked=&q=
GET /api/albums/{id_or_uuid}
GET /api/artists?cursor=&limit=&liked=&q=
GET /api/artists/{id_or_uuid}
GET /api/events?cursor=&limit=&liked=&q=
GET /api/events/{id_or_uuid}
```

`limit` is capped server-side. IDs may be internal numeric IDs or UUIDs.

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
GET /api/tracks/{id_or_uuid}/stream?start_ms=0
GET /api/tracks/{id_or_uuid}/download
```

`stream` returns browser-playback audio in the configured playback format. `start_ms` is optional and defaults to `0`; when present, the stream starts at that millisecond offset relative to the track. For CUE tracks, the offset is relative to the CUE track start and is clamped to the CUE track's duration. Streaming is sequential by default; clients should restart the stream with a new `start_ms` to seek. Add `buffered=true` when the client needs a `Content-Length`/HTTP Range compatible media response, such as Safari or iOS WebView.

`download` returns the original/native track output. Ordinary file downloads support HTTP Range. CUE tracks are rendered as independent audio responses and do not expose the whole-album source file as a frontend URL.

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


## Management

```text
POST /api/artists                  { "name": "Artist" }
POST /api/artists/{id}/aliases     { "alias": "Alias" }
POST /api/artists/merge            { "target": "1", "source": "2", "by_name": false }
POST /api/artists/auto-merge
POST /api/artists/alias-csv-import { "csv": "primary,alias1,alias2\n" }
POST /api/database/vacuum
```
