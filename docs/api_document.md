# Music Library API Documentation

## Base URL
```
http://<your_server_address>:5010/api
```

## Endpoints

### `/add_event`
**Method:** `GET`
**Description:** Add a new event to the library.
**Parameters:**
- `name` (string): Name of the event.
- `year` (string): Year of the event.

**Example Request:**
```
GET /api/add_event?name=SummerFest&year=2023
```

**Example Response:**
```json
{
    "message": "Event added",
    "uuid": "event-uuid",
    "year": 2023
}
```

### `/add_song`
**Method:** `GET`
**Description:** Add a new song to the library.
**Parameters:**
- `name` (string): Name of the song.
- `album_uuid` (string): UUID of the album.
- `artist_uuids` (list of strings): UUIDs of the artists.
- `file_path` (string): Path to the song file.
- `track_number` (int, optional): Track number.
- `disc_number` (int, optional): Disc number.
- `year` (string): Year of the song.

**Example Request:**
```
GET /api/add_song?name=MySong&album_uuid=album-uuid&artist_uuids=artist-uuid1&artist_uuids=artist-uuid2&file_path=/path/to/song.mp3&track_number=1&disc_number=1&year=2023
```

**Example Response:**
```json
{
    "message": "Song added",
    "uuid": "song-uuid",
    "year": 2023
}
```

### `/add_album`
**Method:** `GET`
**Description:** Add a new album to the library.
**Parameters:**
- `name` (string): Name of the album.
- `year` (string): Year of the album.

**Example Request:**
```
GET /api/add_album?name=SummerAlbum&year=2023
```

**Example Response:**
```json
{
    "message": "Album added",
    "uuid": "album-uuid",
    "year": 2023
}
```

### `/add_artist`
**Method:** `GET`
**Description:** Add a new artist to the library.
**Parameters:**
- `name` (string): Name of the artist.

**Example Request:**
```
GET /api/add_artist?name=SummerArtist
```

**Example Response:**
```json
{
    "message": "Artist added",
    "uuid": "artist-uuid"
}
```

### `/like_event/<uuid>`
**Method:** `GET`
**Description:** Like an event by UUID.
**Parameters:**
- `uuid` (string): UUID of the event.

**Example Request:**
```
GET /api/like_event/event-uuid
```

**Example Response:**
```json
{
    "message": "Event liked"
}
```

### `/unlike_event/<uuid>`
**Method:** `GET`
**Description:** Unlike an event by UUID.
**Parameters:**
- `uuid` (string): UUID of the event.

**Example Request:**
```
GET /api/unlike_event/event-uuid
```

**Example Response:**
```json
{
    "message": "Event unliked"
}
```

### `/like_song/<uuid>`
**Method:** `GET`
**Description:** Like a song by UUID.
**Parameters:**
- `uuid` (string): UUID of the song.

**Example Request:**
```
GET /api/like_song/song-uuid
```

**Example Response:**
```json
{
    "message": "Song liked"
}
```

### `/unlike_song/<uuid>`
**Method:** `GET`
**Description:** Unlike a song by UUID.
**Parameters:**
- `uuid` (string): UUID of the song.

**Example Request:**
```
GET /api/unlike_song/song-uuid
```

**Example Response:**
```json
{
    "message": "Song unliked"
}
```

### `/like_album/<uuid>`
**Method:** `GET`
**Description:** Like an album by UUID.
**Parameters:**
- `uuid` (string): UUID of the album.

**Example Request:**
```
GET /api/like_album/album-uuid
```

**Example Response:**
```json
{
    "message": "Album liked"
}
```

### `/unlike_album/<uuid>`
**Method:** `GET`
**Description:** Unlike an album by UUID.
**Parameters:**
- `uuid` (string): UUID of the album.

**Example Request:**
```
GET /api/unlike_album/album-uuid
```

**Example Response:**
```json
{
    "message": "Album unliked"
}
```

### `/like_artist/<uuid>`
**Method:** `GET`
**Description:** Like an artist by UUID.
**Parameters:**
- `uuid` (string): UUID of the artist.

**Example Request:**
```
GET /api/like_artist/artist-uuid
```

**Example Response:**
```json
{
    "message": "Artist liked"
}
```

### `/unlike_artist/<uuid>`
**Method:** `GET`
**Description:** Unlike an artist by UUID.
**Parameters:**
- `uuid` (string): UUID of the artist.

**Example Request:**
```
GET /api/unlike_artist/artist-uuid
```

**Example Response:**
```json
{
    "message": "Artist unliked"
}
```

### `/scan`
**Method:** `GET`
**Description:** Scan a directory for music files.
**Parameters:**
- `directory` (string): Path to the directory to scan.

**Example Request:**
```
GET /api/scan?directory=/path/to/music
```

**Example Response:**
```json
{
    "message": "Scan completed"
}
```

### `/show_event/<uuid>`
**Method:** `GET`
**Description:** Show details of an event by UUID.
**Parameters:**
- `uuid` (string): UUID of the event.

**Example Request:**
```
GET /api/show_event/event-uuid
```

**Example Response:**
```json
{
    "name": "SummerFest",
    "uuid": "event-uuid",
    "year": 2023,
    "albums": [
        {
            "name": "SummerAlbum",
            "uuid": "album-uuid",
            "album_art_path": "/path/to/album/art",
            "album_artists": [
                {
                    "name": "SummerArtist",
                    "uuid": "artist-uuid"
                }
            ]
        }
    ],
    "is_liked": true,
    "liked_time": "2023-01-01T00:00:00Z"
}
```

### `/show_library`
**Method:** `GET`
**Description:** Show details of the entire library.

**Example Request:**
```
GET /api/show_library
```

**Example Response:**
```json
{
    "songs": [
        {
            "name": "MySong",
            "uuid": "song-uuid",
            "artists": [
                {
                    "name": "SummerArtist",
                    "uuid": "artist-uuid"
                }
            ],
            "album": "SummerAlbum",
            "song_art_path": "/path/to/song/art",
            "is_liked": true,
            "liked_time": "2023-01-01T00:00:00Z",
            "file_path": "/path/to/song.mp3"
        }
    ],
    "albums": [
        {
            "name": "SummerAlbum",
            "uuid": "album-uuid",
            "year": 2023,
            "album_art_path": "/path/to/album/art",
            "is_liked": true,
            "liked_time": "2023-01-01T00:00:00Z"
        }
    ],
    "artists": [
        {
            "name": "SummerArtist",
            "uuid": "artist-uuid",
            "artist_art_path": "/path/to/artist/art",
            "is_liked": true,
            "liked_time": "2023-01-01T00:00:00Z"
        }
    ]
}
```

### `/show_liked_events`
**Method:** `GET`
**Description:** Show all liked events.

**Example Request:**
```
GET /api/show_liked_events
```

**Example Response:**
```json
[
    {
        "name": "SummerFest",
        "uuid": "event-uuid",
        "year": 2023,
        "albums": [
            {
                "name": "SummerAlbum",
                "uuid": "album-uuid"
            }
        ],
        "is_liked": true,
        "liked_time": "2023-01-01T00:00:00Z"
    }
]
```

### `/show_liked_songs`
**Method:** `GET`
**Description:** Show all liked songs.

**Example Request:**
```
GET /api/show_liked_songs
```

**Example Response:**
```json
[
    {
        "name": "MySong",
        "uuid": "song-uuid",
        "artists": [
            {
                "name": "SummerArtist",
                "uuid": "artist-uuid"
            }
        ],
        "album": "SummerAlbum",
        "song_art_path": "/path/to/song/art",
        "is_liked": true,
        "liked_time": "2023-01-01T00:00:00Z",
        "file_path": "/path/to/song.mp3"
    }
]
```

### `/show_liked_artists`
**Method:** `GET`
**Description:** Show all liked artists.

**Example Request:**
```
GET /api/show_liked_artists
```

**Example Response:**
```json
[
    {
        "name": "SummerArtist",
        "uuid": "artist-uuid",
        "artist_art_path": "/path/to/artist/art",
        "is_liked": true,
        "liked_time": "2023-01-01T00:00:00Z"
    }
]
```

### `/show_liked_albums`
**Method:** `GET`
**Description:** Show all liked albums.

**Example Request:**
```
GET /api/show_liked_albums
```

**Example Response:**
```json
[
    {
        "name": "SummerAlbum",
        "uuid": "album-uuid",
        "year": 2023,
        "album_art_path": "/path/to/album/art",
        "is_liked": true,
        "liked_time": "2023-01-01T00:00:00Z"
    }
]
```

### `/show_song/<uuid>`
**Method:** `GET`
**Description:** Show details of a song by UUID.
**Parameters:**
- `uuid` (string): UUID of the song.

**Example Request:**
```
GET /api/show_song/song-uuid
```

**Example Response:**
```json
{
    "name": "MySong",
    "uuid": "song-uuid",
    "album": "SummerAlbum",
    "artists": [
        {
            "name": "SummerArtist",
            "uuid": "artist-uuid"
        }
    ],
    "file_path": "/path/to/song.mp3",
    "track_number": 1,
    "disc_number": 1,
    "is_liked": true,
    "liked_time": "2023-01-01T00:00:00Z",
    "song_art_path": "/path/to/song/art",
    "year": 2023,
    "event": "SummerFest"
}
```

### `/show_album/<uuid>`
**Method:** `GET`
**Description:** Show details of an album by UUID.
**Parameters:**
- `uuid` (string): UUID of the album.

**Example Request:**
```
GET /api/show_album/album-uuid
```

**Example Response:**
```json
{
    "name": "SummerAlbum",
    "uuid": "album-uuid",
    "album_artists": [
        {
            "name": "SummerArtist",
            "uuid": "artist-uuid"
        },
        {
            "name": "SummerArtist2",
            "uuid": "artist-uuid2"
        }
    ],
    "songs": [
        {
            "name": "MySong",
            "uuid": "song-uuid",
            "artists": [
                {
                    "name": "SummerArtist",
                    "uuid": "artist-uuid"
                },
                {
                    "name": "SummerArtist2",
                    "uuid": "artist-uuid2"
                }
            ],
            "album": "SummerAlbum",
            "song_art_path": "/path/to/song/art",
            "is_liked": true,
            "track_number": 1,
            "disc_number": 1
        }
    ],
    "is_liked": true,
    "liked_time": "2023-01-01T00:00:00Z",
    "album_art_path": "/path/to/album/art",
    "year": 2023,
    "event": "SummerFest"
}
```

### `/show_artist/<uuid>`
**Method:** `GET`
**Description:** Show details of an artist by UUID.
**Parameters:**
- `uuid` (string): UUID of the artist.

**Example Request:**
```
GET /api/show_artist/artist-uuid
```

**Example Response:**
```json
{
    "name": "SummerArtist",
    "uuid": "artist-uuid",
    "albums": [
        {
            "name": "SummerAlbum",
            "uuid": "album-uuid",
            "year": 2023,
            "album_art_path": "/path/to/album/art",
            "is_liked": true
        }
    ],
    "songs": [
        {
            "name": "MySong",
            "uuid": "song-uuid",
            "artists": [
                {
                    "name": "SummerArtist",
                    "uuid": "artist-uuid"
                }
            ],
            "album": "SummerAlbum",
            "song_art_path": "/path/to/song/art",
            "is_liked": true,
            "track_number": 1,
            "disc_number": 1
        }
    ],
    "is_liked": true,
    "liked_time": "2023-01-01T00:00:00Z",
    "artist_art_path": "/path/to/artist/art"
}
```

### `/search/<query>`
**Method:** `GET`
**Description:** Search for songs, albums, and artists by query.
**Parameters:**
- `query` (string): Search query.

**Example Request:**
```
GET /api/search/Summer
```

**Example Response:**
```json
{
    "songs": [
        {
            "name": "MySong",
            "uuid": "song-uuid",
            "artists": [
                {
                    "name": "SummerArtist",
                    "uuid": "artist-uuid"
                }
            ],
            "album": "SummerAlbum",
            "song_art_path": "/path/to/song/art",
            "is_liked": true,
            "liked_time": "2023-01-01T00:00:00Z",
            "file_path": "/path/to/song.mp3"
        }
    ],
    "albums": [
        {
            "name": "SummerAlbum",
            "uuid": "album-uuid",
            "year": 2023,
            "album_art_path": "/path/to/album/art",
            "is_liked": true,
            "liked_time": "2023-01-01T00:00:00Z"
        }
    ],
    "artists": [
        {
            "name": "SummerArtist",
            "uuid": "artist-uuid",
            "artist_art_path": "/path/to/artist/art",
            "is_liked": true,
            "liked_time": "2023-01-01T00:00:00Z"
        }
    ]
}
```

### `/getfile`
**Method:** `GET`
**Description:** Get a file from the server.
**Parameters:**
- `file_path` (string): Path to the file.

**Example Request:**
```
GET /api/getfile?file_path=/path/to/folder.jpg
```

**Example Response:**
```json
{
    "message": "File not found"
}
```

**Example Response (Successful):**
- File download

### `/getStream`
**Method:** `GET`
**Description:** Stream an audio file.
**Parameters:**
- `file_path` (string): Path to the audio file.

**Example Request:**
```
GET /api/getStream?file_path=/path/to/song.mp3
```

**Example Response:**
- Audio stream

### `/show_relation`
**Method:** `GET`
**Description:** Show relations of an artist by UUID.
**Parameters:**
- `uuid` (string): UUID of the artist.
- `layer` (int): Layer of relations.

**Example Request:**
```
GET /api/show_relation?uuid=artist-uuid&layer=1
```

**Example Response:**
```json
{
    "artist": {
        "name": "SummerArtist",
        "uuid": "artist-uuid",
        "albums": [
            {
                "name": "SummerAlbum",
                "uuid": "album-uuid",
                "songs": [
                    {
                        "name": "MySong",
                        "uuid": "song-uuid"
                    }
                ]
            }
        ]
    }
}
```

### `/merge_artist_by_uuid`
**Method:** `GET`
**Description:** Merge two artists by UUID.
**Parameters:**
- `uuid1` (string): UUID of the first artist.
- `uuid2` (string): UUID of the second artist.

**Example Request:**
```
GET /api/merge_artist_by_uuid?uuid1=artist-uuid1&uuid2=artist-uuid2
```

**Example Response:**
```json
{
    "message": "Artist artist-uuid2 merged into artist-uuid1"
}
```

### `/merge_artist_by_name`
**Method:** `GET`
**Description:** Merge two artists by name.
**Parameters:**
- `name1` (string): Name of the first artist.
- `name2` (string): Name of the second artist.

**Example Request:**
```
GET /api/merge_artist_by_name?name1=SummerArtist1&name2=SummerArtist2
```

**Example Response:**
```json
{
    "message": "Artist SummerArtist2 merged into SummerArtist1"
}
```

### `/auto_merge`
**Method:** `GET`
**Description:** Automatically merge artists based on predefined criteria.

**Example Request:**
```
GET /api/auto_merge
```

**Example Response:**
```json
{
    "message": "Auto merge completed"
}
```

### `/lyrics`
**Method:** `GET`
**Description:** Get lyrics for a song.
**Parameters:**
- `title` (string): Title of the song.
- `artist` (string): Name of the artist.
- `album` (string): Name of the album.
- `duration` (float, optional): Duration of the song.

**Example Request:**
```
GET /api/lyrics?title=MySong&artist=SummerArtist&album=SummerAlbum&duration=300
```

**Example Response (Local Lyrics):**
```json
[
    {
        "title": "MySong",
        "artist": "SummerArtist",
        "lyrics": "Local lyrics here..."
    }
]
```

**Example Response (Aligned Lyrics):**
```json
[
    {
        "title": "MySong",
        "artist": "SummerArtist",
        "lyrics": "Aligned lyrics here..."
    }
]
```

**Example Response (No Lyrics):**
```json
{}
```