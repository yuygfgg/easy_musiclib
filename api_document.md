# Music Library API Documentation

This document provides an overview of the endpoints available in the Music Library API, which allows you to manage and interact with a music library. The API is built using Flask and supports CORS.

## Base URL

```
http://<your-server>:<port>
```

## Endpoints

### Add Song

**URL:** `/add_song`

**Method:** `GET`

**Parameters:**

- `name` (string): Name of the song.
- `album_uuid` (string): UUID of the album.
- `artist_uuids` (list of strings): List of artist UUIDs.
- `file_path` (string): File path of the song.
- `track_number` (integer, optional): Track number of the song. Default is 1.
- `disc_number` (integer, optional): Disc number of the song. Default is 1.
- `year` (string, optional): Year of the song.

**Response:**

```json
{
    "message": "Song added",
    "uuid": "song-uuid",
    "year": "year"
}
```

### Add Album

**URL:** `/add_album`

**Method:** `GET`

**Parameters:**

- `name` (string): Name of the album.
- `year` (string, optional): Year of the album.

**Response:**

```json
{
    "message": "Album added",
    "uuid": "album-uuid",
    "year": "year"
}
```

### Add Artist

**URL:** `/add_artist`

**Method:** `GET`

**Parameters:**

- `name` (string): Name of the artist.

**Response:**

```json
{
    "message": "Artist added",
    "uuid": "artist-uuid"
}
```

### Like Song

**URL:** `/like_song/<uuid>`

**Method:** `GET`

**Response:**

```json
{
    "message": "Song liked"
}
```

### Unlike Song

**URL:** `/unlike_song/<uuid>`

**Method:** `GET`

**Response:**

```json
{
    "message": "Song unliked"
}
```

### Like Album

**URL:** `/like_album/<uuid>`

**Method:** `GET`

**Response:**

```json
{
    "message": "Album liked"
}
```

### Unlike Album

**URL:** `/unlike_album/<uuid>`

**Method:** `GET`

**Response:**

```json
{
    "message": "Album unliked"
}
```

### Like Artist

**URL:** `/like_artist/<uuid>`

**Method:** `GET`

**Response:**

```json
{
    "message": "Artist liked"
}
```

### Unlike Artist

**URL:** `/unlike_artist/<uuid>`

**Method:** `GET`

**Response:**

```json
{
    "message": "Artist unliked"
}
```

### Scan Directory

**URL:** `/scan`

**Method:** `GET`

**Parameters:**

- `directory` (string): Directory path to scan for music files.

**Response:**

```json
{
    "message": "Scan completed"
}
```

### Search Song

**URL:** `/search_song/<name>`

**Method:** `GET`

**Response:**

```json
{
    "name": "song name",
    "uuid": "song-uuid"
}
```

**Error Response:**

```json
{
    "message": "Song not found"
}
```

### Search Album

**URL:** `/search_album/<name>`

**Method:** `GET`

**Response:**

```json
{
    "name": "album name",
    "uuid": "album-uuid"
}
```

**Error Response:**

```json
{
    "message": "Album not found"
}
```

### Search Artist

**URL:** `/search_artist/<name>`

**Method:** `GET`

**Response:**

```json
{
    "name": "artist name",
    "uuid": "artist-uuid"
}
```

**Error Response:**

```json
{
    "message": "Artist not found"
}
```

### Show Library

**URL:** `/show_library`

**Method:** `GET`

**Response:**

```json
{
    "artists": {
        "uuid1": "artist name 1",
        "uuid2": "artist name 2"
    },
    "albums": {
        "uuid1": "album name 1",
        "uuid2": "album name 2"
    },
    "songs": {
        "uuid1": "song name 1",
        "uuid2": "song name 2"
    }
}
```

### Show Liked Songs

**URL:** `/show_liked_songs`

**Method:** `GET`

**Response:**

```json
[
    {
        "name": "song name",
        "uuid": "song-uuid",
        "artists": ["artist1", "artist2"],
        "album": "album name",
        "song_art_path": "path/to/art",
        "is_liked": true,
        "file_path": "path/to/song"
    }
]
```

### Show Liked Artists

**URL:** `/show_liked_artists`

**Method:** `GET`

**Response:**

```json
[
    {
        "name": "artist name",
        "uuid": "artist-uuid",
        "artist_art_path": "path/to/art",
        "is_liked": true
    }
]
```

### Show Liked Albums

**URL:** `/show_liked_albums`

**Method:** `GET`

**Response:**

```json
[
    {
        "name": "album name",
        "uuid": "album-uuid",
        "year": "year",
        "album_art_path": "path/to/art",
        "is_liked": true
    }
]
```

### Show Song

**URL:** `/show_song/<uuid>`

**Method:** `GET`

**Response:**

```json
{
    "name": "song name",
    "uuid": "song-uuid",
    "album": "album name",
    "artists": ["artist1", "artist2"],
    "file_path": "path/to/song",
    "track_number": 1,
    "disc_number": 1,
    "is_liked": true,
    "song_art_path": "path/to/art",
    "year": "year"
}
```

**Error Response:**

```json
{
    "message": "Song not found"
}
```

### Show Album

**URL:** `/show_album/<uuid>`

**Method:** `GET`

**Response:**

```json
{
    "name": "album name",
    "uuid": "album-uuid",
    "album_artists": [
        {
            "name": "artist name",
            "uuid": "artist-uuid"
        }
    ],
    "songs": [
        {
            "name": "song name",
            "uuid": "song-uuid",
            "artists": ["artist1", "artist2"],
            "album": "album name",
            "song_art_path": "path/to/art",
            "is_liked": true,
            "track_number": 1,
            "disc_number": 1
        }
    ],
    "is_liked": true,
    "album_art_path": "path/to/art",
    "year": "year"
}
```

**Error Response:**

```json
{
    "message": "Album not found"
}
```

### Show Artist

**URL:** `/show_artist/<uuid>`

**Method:** `GET`

**Response:**

```json
{
    "name": "artist name",
    "uuid": "artist-uuid",
    "albums": [
        {
            "name": "album name",
            "uuid": "album-uuid",
            "year": "year",
            "album_art_path": "path/to/art",
            "is_liked": true
        }
    ],
    "songs": [
        {
            "name": "song name",
            "uuid": "song-uuid",
            "artists": ["artist1", "artist2"],
            "album": "album name",
            "song_art_path": "path/to/art",
            "is_liked": true,
            "track_number": 1,
            "disc_number": 1
        }
    ],
    "is_liked": true,
    "artist_art_path": "path/to/art"
}
```

**Error Response:**

```json
{
    "message": "Artist not found"
}
```

### Fuzzy Search

**URL:** `/search/<query>`

**Method:** `GET`

**Response:**

```json
{
    "songs": [
        {
            "name": "song name",
            "uuid": "song-uuid",
            "artists": ["artist1", "artist2"],
            "album": "album name",
            "song_art_path": "path/to/art",
            "is_liked": true,
            "file_path": "path/to/song"
        }
    ],
    "albums": [
        {
            "name": "album name",
            "uuid": "album-uuid",
            "year": "year",
            "album_art_path": "path/to/art",
            "is_liked": true
        }
    ],
    "artists": [
        {
            "name": "artist name",
            "uuid": "artist-uuid",
            "artist_art_path": "path/to/art",
            "is_liked": true
        }
    ]
}
```

### Get File

**URL:** `/getfile`

**Method:** `GET`

**Parameters:**

- `file_path` (string): Path to the file to be retrieved.

**Response:**

- Returns the requested file as an attachment.

**Error Response:**

```json
{
    "message": "File not found"
}
```

## Error Handling

The API uses standard HTTP status codes to indicate the success or failure of an API request. Common status codes include:

- `200 OK`: The request was successful.
- `201 Created`: The resource was successfully created.
- `400 Bad Request`: The request was invalid or cannot be served.
- `404 Not Found`: The requested resource could not be found.
- `500 Internal Server Error`: An error occurred on the server.

## Example Usage

### Adding a Song

```bash
curl "http://<your-server>:<port>/add_song?name=SongName&album_uuid=album-uuid&artist_uuids=artist-uuid&file_path=path/to/song.mp3"
```

### Searching for an Album

```bash
curl "http://<your-server>:<port>/search_album/AlbumName"
```

### Showing the Entire Library

```bash
curl "http://<your-server>:<port>/show_library"
```

This documentation provides an overview of the various endpoints and their usage. For more detailed information or any specific requirements, please refer to the source code or contact the API maintainer.