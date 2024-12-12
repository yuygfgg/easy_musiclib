# Music Library API Documentation

Base URL: `http://localhost:5010/api`

## Artists

### Add Artist
- **Endpoint**: `/add_artist`
- **Method**: GET
- **Parameters**: 
  - `name` (string): Artist name
- **Example Request**: 
  ```
  GET /api/add_artist?name=Taylor%20Swift
  ```
- **Example Response**:
  ```json
  {
    "message": "Artist added",
    "uuid": "550e8400-e29b-41d4-a716-446655440000"
  }
  ```
- **Status Codes**: 201 Created

### Like Artist
- **Endpoint**: `/like_artist/<uuid>`
- **Method**: GET
- **Example Request**: 
  ```
  GET /api/like_artist/550e8400-e29b-41d4-a716-446655440000
  ```
- **Example Response**:
  ```json
  {
    "message": "Artist liked"
  }
  ```
- **Status Codes**: 200 OK

### Unlike Artist
- **Endpoint**: `/unlike_artist/<uuid>`
- **Method**: GET
- **Example Request**: 
  ```
  GET /api/unlike_artist/550e8400-e29b-41d4-a716-446655440000
  ```
- **Example Response**:
  ```json
  {
    "message": "Artist unliked"
  }
  ```
- **Status Codes**: 200 OK

### Show Artist
- **Endpoint**: `/show_artist/<uuid>`
- **Method**: GET
- **Example Request**: 
  ```
  GET /api/show_artist/550e8400-e29b-41d4-a716-446655440000
  ```
- **Example Response**:
  ```json
  {
    "name": "Taylor Swift",
    "uuid": "550e8400-e29b-41d4-a716-446655440000",
    "albums": [
      {
        "name": "1989",
        "uuid": "660e8400-e29b-41d4-a716-446655440000",
        "year": 2014,
        "date": "2014-10-27",
        "album_art_path": "/path/to/art.jpg",
        "is_liked": true
      }
    ],
    "songs": [
      {
        "name": "Shake It Off",
        "uuid": "770e8400-e29b-41d4-a716-446655440000",
        "artists": [{"name": "Taylor Swift", "uuid": "550e8400-e29b-41d4-a716-446655440000"}],
        "album": {"name": "1989", "uuid": "660e8400-e29b-41d4-a716-446655440000"},
        "song_art_path": "/path/to/song.jpg",
        "is_liked": true,
        "track_number": 1,
        "disc_number": 1
      }
    ],
    "is_liked": true,
    "liked_time": "2024-01-01T12:00:00",
    "artist_art_path": "/path/to/artist.jpg"
  }
  ```
- **Status Codes**: 200 OK, 404 Not Found

## Albums

### Like Album
- **Endpoint**: `/like_album/<uuid>`
- **Method**: GET
- **Example Request**: 
  ```
  GET /api/like_album/660e8400-e29b-41d4-a716-446655440000
  ```
- **Example Response**:
  ```json
  {
    "message": "Album liked"
  }
  ```
- **Status Codes**: 200 OK

### Unlike Album
- **Endpoint**: `/unlike_album/<uuid>`
- **Method**: GET
- **Example Request**: 
  ```
  GET /api/unlike_album/660e8400-e29b-41d4-a716-446655440000
  ```
- **Example Response**:
  ```json
  {
    "message": "Album unliked"
  }
  ```
- **Status Codes**: 200 OK

### Show Album
- **Endpoint**: `/show_album/<uuid>`
- **Method**: GET
- **Example Request**: 
  ```
  GET /api/show_album/660e8400-e29b-41d4-a716-446655440000
  ```
- **Example Response**:
  ```json
  {
    "name": "1989",
    "uuid": "660e8400-e29b-41d4-a716-446655440000",
    "album_artists": [
      {
        "name": "Taylor Swift",
        "uuid": "550e8400-e29b-41d4-a716-446655440000"
      }
    ],
    "songs": [
      {
        "name": "Shake It Off",
        "uuid": "770e8400-e29b-41d4-a716-446655440000",
        "artists": [{"name": "Taylor Swift", "uuid": "550e8400-e29b-41d4-a716-446655440000"}],
        "album": {"name": "1989", "uuid": "660e8400-e29b-41d4-a716-446655440000"},
        "song_art_path": "/path/to/song.jpg",
        "is_liked": true,
        "track_number": 1,
        "disc_number": 1
      }
    ],
    "is_liked": true,
    "liked_time": "2024-01-01T12:00:00",
    "album_art_path": "/path/to/album.jpg",
    "year": 2014,
    "date": "2014-10-27",
    "event": {"name": "Album Release", "uuid": "880e8400-e29b-41d4-a716-446655440000"}
  }
  ```
- **Status Codes**: 200 OK, 404 Not Found

## Songs

### Like Song
- **Endpoint**: `/like_song/<uuid>`
- **Method**: GET
- **Example Request**: 
  ```
  GET /api/like_song/770e8400-e29b-41d4-a716-446655440000
  ```
- **Example Response**:
  ```json
  {
    "message": "Song liked"
  }
  ```
- **Status Codes**: 200 OK

### Unlike Song
- **Endpoint**: `/unlike_song/<uuid>`
- **Method**: GET
- **Example Request**: 
  ```
  GET /api/unlike_song/770e8400-e29b-41d4-a716-446655440000
  ```
- **Example Response**:
  ```json
  {
    "message": "Song unliked"
  }
  ```
- **Status Codes**: 200 OK

### Show Song
- **Endpoint**: `/show_song/<uuid>`
- **Method**: GET
- **Example Request**: 
  ```
  GET /api/show_song/770e8400-e29b-41d4-a716-446655440000
  ```
- **Example Response**:
  ```json
  {
    "name": "Shake It Off",
    "uuid": "770e8400-e29b-41d4-a716-446655440000",
    "album": {"name": "1989", "uuid": "660e8400-e29b-41d4-a716-446655440000"},
    "artists": [{"name": "Taylor Swift", "uuid": "550e8400-e29b-41d4-a716-446655440000"}],
    "file_path": "/path/to/song.mp3",
    "track_number": 1,
    "disc_number": 1,
    "is_liked": true,
    "liked_time": "2024-01-01T12:00:00",
    "song_art_path": "/path/to/song.jpg",
    "year": 2014,
    "date": "2014-10-27",
    "event": {"name": "Single Release", "uuid": "880e8400-e29b-41d4-a716-446655440000"}
  }
  ```
- **Status Codes**: 200 OK, 404 Not Found

## Library Management

### Scan Directory
- **Endpoint**: `/scan`
- **Method**: GET
- **Parameters**:
  - `directory` (string): Path to music directory
- **Example Request**: 
  ```
  GET /api/scan?directory=/path/to/music
  ```
- **Example Response**:
  ```json
  {
    "message": "Scan completed"
  }
  ```
- **Status Codes**: 200 OK, 400 Bad Request

### Show Library
- **Endpoint**: `/show_library`
- **Method**: GET
- **Example Request**: 
  ```
  GET /api/show_library
  ```
- **Example Response**:
  ```json
  {
    "songs": [
      {
        "name": "Shake It Off",
        "uuid": "770e8400-e29b-41d4-a716-446655440000",
        "artists": [{"name": "Taylor Swift", "uuid": "550e8400-e29b-41d4-a716-446655440000"}],
        "album": {"name": "1989", "uuid": "660e8400-e29b-41d4-a716-446655440000"},
        "song_art_path": "/path/to/song.jpg",
        "is_liked": true,
        "liked_time": "2024-01-01T12:00:00",
        "file_path": "/path/to/song.mp3"
      }
    ],
    "albums": [...],
    "artists": [...]
  }
  ```
- **Status Codes**: 200 OK

## Events

### Like Event
- **Endpoint**: `/like_event/<uuid>`
- **Method**: GET
- **Example Request**: 
  ```
  GET /api/like_event/880e8400-e29b-41d4-a716-446655440000
  ```
- **Example Response**:
  ```json
  {
    "message": "Event liked"
  }
  ```
- **Status Codes**: 200 OK, 404 Not Found

### Unlike Event
- **Endpoint**: `/unlike_event/<uuid>`
- **Method**: GET
- **Example Request**: 
  ```
  GET /api/unlike_event/880e8400-e29b-41d4-a716-446655440000
  ```
- **Example Response**:
  ```json
  {
    "message": "Event unliked"
  }
  ```
- **Status Codes**: 200 OK, 404 Not Found

### Show Event
- **Endpoint**: `/show_event/<uuid>`
- **Method**: GET
- **Example Request**: 
  ```
  GET /api/show_event/880e8400-e29b-41d4-a716-446655440000
  ```
- **Example Response**:
  ```json
  {
    "name": "Album Release",
    "uuid": "880e8400-e29b-41d4-a716-446655440000",
    "year": 2014,
    "date": "2014-10-27",
    "albums": [
      {
        "name": "1989",
        "uuid": "660e8400-e29b-41d4-a716-446655440000",
        "album_art_path": "/path/to/album.jpg",
        "album_artists": [
          {
            "name": "Taylor Swift",
            "uuid": "550e8400-e29b-41d4-a716-446655440000"
          }
        ]
      }
    ],
    "is_liked": true,
    "liked_time": "2024-01-01T12:00:00"
  }
  ```
- **Status Codes**: 200 OK, 404 Not Found

## Search and Relations

### Search
- **Endpoint**: `/search/<query>`
- **Method**: GET
- **Example Request**: 
  ```
  GET /api/search/taylor
  ```
- **Example Response**:
  ```json
  {
    "songs": [...],
    "albums": [...],
    "artists": [
      {
        "name": "Taylor Swift",
        "uuid": "550e8400-e29b-41d4-a716-446655440000",
        "artist_art_path": "/path/to/artist.jpg",
        "is_liked": true,
        "liked_time": "2024-01-01T12:00:00"
      }
    ]
  }
  ```
- **Status Codes**: 200 OK

### Show Relation
- **Endpoint**: `/show_relation`
- **Method**: GET
- **Parameters**:
  - `uuid` (string): Artist UUID
  - `layer` (integer): Depth of relations to show
- **Example Request**: 
  ```
  GET /api/show_relation?uuid=550e8400-e29b-41d4-a716-446655440000&layer=2
  ```
- **Example Response**:
  ```json
  {
    "artist": "Taylor Swift",
    "relations": [
      {
        "artist": "Ed Sheeran",
        "strength": 5,
        "relations": []
      }
    ]
  }
  ```
- **Status Codes**: 200 OK, 400 Bad Request

## File Operations

### Get File
- **Endpoint**: `/getfile`
- **Method**: GET
- **Parameters**:
  - `file_path` (string): Path to file
- **Example Request**: 
  ```
  GET /api/getfile?file_path=/path/to/cover.jpg
  ```
- **Response**: Image file
- **Status Codes**: 200 OK, 404 Not Found, 400 Bad Request

### Get Stream
- **Endpoint**: `/getStream`
- **Method**: GET
- **Parameters**:
  - `file_path` (string): Path to audio file
- **Example Request**: 
  ```
  GET /api/getStream?file_path=/path/to/song.mp3
  ```
- **Response**: Audio stream
- **Status Codes**: 200 OK, 404 Not Found, 400 Bad Request

## Lyrics

### Get Lyrics
- **Endpoint**: `/lyrics`
- **Method**: GET
- **Parameters**:
  - `title` (string): Song title
  - `artist` (string): Artist name
  - `album` (string): Album name
  - `duration` (float): Song duration in seconds
- **Example Request**: 
  ```
  GET /api/lyrics?title=Shake%20It%20Off&artist=Taylor%20Swift&album=1989&duration=219.5
  ```
- **Example Response**:
  ```json
  {
    "lyrics": [
      {
        "text": "I stay out too late",
        "time": 15.5
      },
      {
        "text": "Got nothing in my brain",
        "time": 17.8
      }
    ]
  }
  ```
- **Status Codes**: 200 OK, 404 Not Found

## Liked Items

### Show Liked Events
- **Endpoint**: `/show_liked_events`
- **Method**: GET
- **Example Request**: 
  ```
  GET /api/show_liked_events
  ```
- **Example Response**:
  ```json
  [
    {
      "name": "Album Release",
      "uuid": "880e8400-e29b-41d4-a716-446655440000",
      "year": 2014,
      "date": "2014-10-27",
      "albums": [
        {
          "name": "1989",
          "uuid": "660e8400-e29b-41d4-a716-446655440000"
        }
      ],
      "is_liked": true,
      "liked_time": "2024-01-01T12:00:00"
    }
  ]
  ```
- **Status Codes**: 200 OK

### Show Liked Songs
- **Endpoint**: `/show_liked_songs`
- **Method**: GET
- **Example Request**: 
  ```
  GET /api/show_liked_songs
  ```
- **Example Response**:
  ```json
  [
    {
      "name": "Shake It Off",
      "uuid": "770e8400-e29b-41d4-a716-446655440000",
      "artists": [{"name": "Taylor Swift", "uuid": "550e8400-e29b-41d4-a716-446655440000"}],
      "album": {"name": "1989", "uuid": "660e8400-e29b-41d4-a716-446655440000"},
      "song_art_path": "/path/to/song.jpg",
      "is_liked": true,
      "liked_time": "2024-01-01T12:00:00",
      "file_path": "/path/to/song.mp3"
    }
  ]
  ```
- **Status Codes**: 200 OK

### Show Liked Artists
- **Endpoint**: `/show_liked_artists`
- **Method**: GET
- **Example Request**: 
  ```
  GET /api/show_liked_artists
  ```
- **Example Response**:
  ```json
  [
    {
      "name": "Taylor Swift",
      "uuid": "550e8400-e29b-41d4-a716-446655440000",
      "artist_art_path": "/path/to/artist.jpg",
      "is_liked": true,
      "liked_time": "2024-01-01T12:00:00"
    }
  ]
  ```
- **Status Codes**: 200 OK

### Show Liked Albums
- **Endpoint**: `/show_liked_albums`
- **Method**: GET
- **Example Request**: 
  ```
  GET /api/show_liked_albums
  ```
- **Example Response**:
  ```json
  [
    {
      "name": "1989",
      "uuid": "660e8400-e29b-41d4-a716-446655440000",
      "year": 2014,
      "date": "2014-10-27",
      "album_art_path": "/path/to/album.jpg",
      "is_liked": true,
      "liked_time": "2024-01-01T12:00:00"
    }
  ]
  ```
- **Status Codes**: 200 OK

## Artist Merging

### Merge Artists by UUID
- **Endpoint**: `/merge_artist_by_uuid`
- **Method**: GET
- **Parameters**:
  - `uuid1` (string): UUID of first artist (target)
  - `uuid2` (string): UUID of second artist (to be merged)
- **Example Request**: 
  ```
  GET /api/merge_artist_by_uuid?uuid1=550e8400-e29b-41d4-a716-446655440000&uuid2=550e8400-e29b-41d4-a716-446655440001
  ```
- **Example Response**:
  ```json
  {
    "message": "Artist 550e8400-e29b-41d4-a716-446655440001 merged into 550e8400-e29b-41d4-a716-446655440000"
  }
  ```
- **Status Codes**: 200 OK, 400 Bad Request

### Merge Artists by Name
- **Endpoint**: `/merge_artist_by_name`
- **Method**: GET
- **Parameters**:
  - `name1` (string): Name of first artist (target)
  - `name2` (string): Name of second artist (to be merged)
- **Example Request**: 
  ```
  GET /api/merge_artist_by_name?name1=Taylor%20Swift&name2=T.%20Swift
  ```
- **Example Response**:
  ```json
  {
    "message": "Artist T. Swift merged into Taylor Swift"
  }
  ```
- **Status Codes**: 200 OK, 400 Bad Request

### Auto Merge Artists
- **Endpoint**: `/auto_merge`
- **Method**: GET
- **Example Request**: 
  ```
  GET /api/auto_merge
  ```
- **Example Response**:
  ```json
  {
    "message": "Auto merge completed"
  }
  ```
- **Status Codes**: 200 OK

Additionally, there are two static file serving endpoints:

## Static Files

### Serve Index
- **Endpoint**: `/`
- **Method**: GET
- **Response**: Returns the index.html file from the webui directory

### Serve Static Files
- **Endpoint**: `/<path:path>`
- **Method**: GET
- **Response**: Returns the requested static file from the webui directory
