## Music Library API 文档

### 基本信息

- **Base URL**: `http://127.0.0.1:5000`
- **Content-Type**: `application/json`
- **方法**: GET

### 端点

#### 1. 添加歌曲

- **URL**: `/add_song`
- **方法**: GET
- **参数**:
  - `name` (str): 歌曲名称
  - `album_uuid` (str): 专辑 UUID
  - `artist_uuids` (list): 艺术家 UUID 列表
  - `file_path` (str): 文件路径
  - `track_number` (int, 可选): 曲目编号，默认值为 1
  - `disc_number` (int, 可选): 光盘编号，默认值为 1
- **示例**:
  ```sh
  curl "http://127.0.0.1:5000/add_song?name=SongName&album_uuid=album-uuid&artist_uuids=artist1-uuid&artist_uuids=artist2-uuid&file_path=/path/to/file.mp3&track_number=1&disc_number=1"
  ```
- **响应**:
  ```json
  {
    "message": "Song added"
  }
  ```

#### 2. 添加专辑

- **URL**: `/add_album`
- **方法**: GET
- **参数**:
  - `name` (str): 专辑名称
- **示例**:
  ```sh
  curl "http://127.0.0.1:5000/add_album?name=AlbumName"
  ```
- **响应**:
  ```json
  {
    "message": "Album added",
    "uuid": "album-uuid"
  }
  ```

#### 3. 添加艺术家

- **URL**: `/add_artist`
- **方法**: GET
- **参数**:
  - `name` (str): 艺术家名称
- **示例**:
  ```sh
  curl "http://127.0.0.1:5000/add_artist?name=ArtistName"
  ```
- **响应**:
  ```json
  {
    "message": "Artist added",
    "uuid": "artist-uuid"
  }
  ```

#### 4. 搜索歌曲

- **URL**: `/search_song/<name>`
- **方法**: GET
- **参数**: 无
- **示例**:
  ```sh
  curl "http://127.0.0.1:5000/search_song/SongName"
  ```
- **响应**:
  ```json
  {
    "name": "SongName",
    "uuid": "song-uuid"
  }
  ```

#### 5. 搜索专辑

- **URL**: `/search_album/<name>`
- **方法**: GET
- **参数**: 无
- **示例**:
  ```sh
  curl "http://127.0.0.1:5000/search_album/AlbumName"
  ```
- **响应**:
  ```json
  {
    "name": "AlbumName",
    "uuid": "album-uuid"
  }
  ```

#### 6. 搜索艺术家

- **URL**: `/search_artist/<name>`
- **方法**: GET
- **参数**: 无
- **示例**:
  ```sh
  curl "http://127.0.0.1:5000/search_artist/ArtistName"
  ```
- **响应**:
  ```json
  {
    "name": "ArtistName",
    "uuid": "artist-uuid"
  }
  ```

#### 7. 喜欢歌曲

- **URL**: `/like_song/<uuid>`
- **方法**: GET
- **参数**: 无
- **示例**:
  ```sh
  curl "http://127.0.0.1:5000/like_song/song-uuid"
  ```
- **响应**:
  ```json
  {
    "message": "Song liked"
  }
  ```

#### 8. 取消喜欢歌曲

- **URL**: `/unlike_song/<uuid>`
- **方法**: GET
- **参数**: 无
- **示例**:
  ```sh
  curl "http://127.0.0.1:5000/unlike_song/song-uuid"
  ```
- **响应**:
  ```json
  {
    "message": "Song unliked"
  }
  ```

#### 9. 喜欢专辑

- **URL**: `/like_album/<uuid>`
- **方法**: GET
- **参数**: 无
- **示例**:
  ```sh
  curl "http://127.0.0.1:5000/like_album/album-uuid"
  ```
- **响应**:
  ```json
  {
    "message": "Album liked"
  }
  ```

#### 10. 取消喜欢专辑

- **URL**: `/unlike_album/<uuid>`
- **方法**: GET
- **参数**: 无
- **示例**:
  ```sh
  curl "http://127.0.0.1:5000/unlike_album/album-uuid"
  ```
- **响应**:
  ```json
  {
    "message": "Album unliked"
  }
  ```

#### 11. 喜欢艺术家

- **URL**: `/like_artist/<uuid>`
- **方法**: GET
- **参数**: 无
- **示例**:
  ```sh
  curl "http://127.0.0.1:5000/like_artist/artist-uuid"
  ```
- **响应**:
  ```json
  {
    "message": "Artist liked"
  }
  ```

#### 12. 取消喜欢艺术家

- **URL**: `/unlike_artist/<uuid>`
- **方法**: GET
- **参数**: 无
- **示例**:
  ```sh
  curl "http://127.0.0.1:5000/unlike_artist/artist-uuid"
  ```
- **响应**:
  ```json
  {
    "message": "Artist unliked"
  }
  ```

#### 13. 展示整个音乐库

- **URL**: `/show_library`
- **方法**: GET
- **参数**: 无
- **示例**:
  ```sh
  curl "http://127.0.0.1:5000/show_library"
  ```
- **响应**:
  ```json
  {
    "artists": {
      "artist-uuid1": "ArtistName1",
      "artist-uuid2": "ArtistName2"
    },
    "albums": {
      "album-uuid1": "AlbumName1",
      "album-uuid2": "AlbumName2"
    },
    "songs": {
      "song-uuid1": "SongName1",
      "song-uuid2": "SongName2"
    }
  }
  ```

#### 14. 展示喜欢的歌曲

- **URL**: `/show_liked_songs`
- **方法**: GET
- **参数**: 无
- **示例**:
  ```sh
  curl "http://127.0.0.1:5000/show_liked_songs"
  ```
- **响应**:
  ```json
  [
    {
      "name": "SongName1",
      "uuid": "song-uuid1"
    },
    {
      "name": "SongName2",
      "uuid": "song-uuid2"
    }
  ]
  ```

#### 15. 展示喜欢的艺术家

- **URL**: `/show_liked_artists`
- **方法**: GET
- **参数**: 无
- **示例**:
  ```sh
  curl "http://127.0.0.1:5000/show_liked_artists"
  ```
- **响应**:
  ```json
  [
    {
      "name": "ArtistName1",
      "uuid": "artist-uuid1"
    },
    {
      "name": "ArtistName2",
      "uuid": "artist-uuid2"
    }
  ]
  ```

#### 16. 展示喜欢的专辑

- **URL**: `/show_liked_albums`
- **方法**: GET
- **参数**: 无
- **示例**:
  ```sh
  curl "http://127.0.0.1:5000/show_liked_albums"
  ```
- **响应**:
  ```json
  [
    {
      "name": "AlbumName1",
      "uuid": "album-uuid1"
    },
    {
      "name": "AlbumName2",
      "uuid": "album-uuid2"
    }
  ]
  ```

#### 17. 扫描音乐目录

- **URL**: `/scan`
- **方法**: GET
- **参数**:
  - `directory` (str): 要扫描的目录路径
- **示例**:
  ```sh
  curl "http://127.0.0.1:5000/scan?directory=/path/to/music"
  ```
- **响应**:
  ```json
  {
    "message": "Scan completed"
  }
  ```

#### 18. 展示歌曲信息

- **URL**: `/show_song/<uuid>`
- **方法**: GET
- **参数**: 无
- **示例**:
  ```sh
  curl "http://127.0.0.1:5000/show_song/song-uuid"
  ```
- **响应**:
  ```json
  {
    "name": "SongName",
    "uuid": "song-uuid",
    "album": {
      "name": "AlbumName",
      "uuid": "album-uuid"
    },
    "artists": [
      {
        "name": "ArtistName1",
        "uuid": "artist-uuid1"
      },
      {
        "name": "ArtistName2",
        "uuid": "artist-uuid2"
      }
    ],
    "file_path": "/path/to/song/file.mp3",
    "track_number": 1,
    "disc_number": 1,
    "is_liked": false,
    "song_art_path": "/path/to/song/art.jpg"
  }
  ```

#### 19. 展示专辑信息

- **URL**: `/show_album/<uuid>`
- **方法**: GET
- **参数**: 无
- **示例**:
  ```sh
  curl "http://127.0.0.1:5000/show_album/album-uuid"
  ```
- **响应**:
  ```json
  {
    "name": "AlbumName",
    "uuid": "album-uuid",
    "album_artists": [
      {
        "name": "ArtistName1",
        "uuid": "artist-uuid1"
      },
      {
        "name": "ArtistName2",
        "uuid": "artist-uuid2"
      }
    ],
    "songs": [
      {
        "name": "SongName1",
        "uuid": "song-uuid1",
        "track_number": 1,
        "disc_number": 1
      },
      {
        "name": "SongName2",
        "uuid": "song-uuid2",
        "track_number": 2,
        "disc_number": 1
      }
    ],
    "is_liked": false,
    "album_art_path": "/path/to/album/art.jpg"
  }
  ```

#### 20. 展示艺术家信息

- **URL**: `/show_artist/<uuid>`
- **方法**: GET
- **参数**: 无
- **示例**:
  ```sh
  curl "http://127.0.0.1:5000/show_artist/artist-uuid"
  ```
- **响应**:
  ```json
  {
    "name": "ArtistName",
    "uuid": "artist-uuid",
    "albums": [
      {
        "name": "AlbumName1",
        "uuid": "album-uuid1"
      },
      {
        "name": "AlbumName2",
        "uuid": "album-uuid2"
      }
    ],
    "songs": [
      {
        "name": "SongName1",
        "uuid": "song-uuid1",
        "track_number": 1,
        "disc_number": 1
      },
      {
        "name": "SongName2",
        "uuid": "song-uuid2",
        "track_number": 2,
        "disc_number": 1
      }
    ],
    "is_liked": false,
    "artist_art_path": "/path/to/artist/art.jpg"
  }
  ```

#### 21. 模糊搜索

- **URL**: `/search/<query>`
- **方法**: GET
- **参数**: 无
- **示例**:
  ```sh
  curl "http://127.0.0.1:5000/search/Query"
  ```
- **响应**:
  ```json
  {
    "songs": [
      {
        "name": "SongName",
        "uuid": "song-uuid"
      }
    ],
    "albums": [
      {
        "name": "AlbumName",
        "uuid": "album-uuid"
      }
    ],
    "artists": [
      {
        "name": "ArtistName",
        "uuid": "artist-uuid"
      }
    ]
  }
  ```

#### 22. 获取文件

- **URL**: `/getfile`
- **方法**: GET
- **参数**:
  - `file_path` (str): 文件路径
- **示例**:
  ```sh
  curl "http://127.0.0.1:5000/getfile?file_path=/path/to/file.mp3"
  ```
- **响应**: 直接返回文件内容。
