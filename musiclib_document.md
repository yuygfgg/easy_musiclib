# `musiclib.py` 使用说明

## 简介

`musiclib.py` 是一个用于管理音乐库的 Python 脚本。它可以扫描指定目录下的音乐文件（支持 `.mp3` 和 `.flac` 格式），并提取 ID3 标签信息，将音乐组织成艺术家、专辑和歌曲的层级结构。同时，提供了对艺术家、专辑和歌曲的点赞/取消点赞功能，并可以显示和搜索音乐库中的内容。

## 类和方法

### 类 `Artist`

表示音乐艺术家。

#### 属性

- `name`: 艺术家名称（小写）
- `uuid`: 艺术家的唯一 ID
- `is_liked`: 是否被点赞
- `artist_art_path`: 艺术家图片路径

#### 方法

- `like()`: 点赞艺术家
- `unlike()`: 取消点赞艺术家

### 类 `Album`

表示音乐专辑。

#### 属性

- `name`: 专辑名称
- `uuid`: 专辑的唯一 ID
- `album_artists`: 专辑的艺术家集合
- `songs`: 专辑中的歌曲列表
- `is_liked`: 是否被点赞
- `album_art_path`: 专辑图片路径

#### 方法

- `like()`: 点赞专辑
- `unlike()`: 取消点赞专辑

### 类 `Song`

表示单曲。

#### 属性

- `name`: 歌曲名称
- `uuid`: 歌曲的唯一 ID
- `album`: 歌曲所属专辑的信息
- `artists`: 歌曲的艺术家列表
- `file_path`: 歌曲文件路径
- `track_number`: 歌曲在专辑中的轨道号
- `disc_number`: 歌曲所在的碟片号
- `is_liked`: 是否被点赞
- `song_art_path`: 歌曲图片路径

#### 方法

- `like()`: 点赞歌曲
- `unlike()`: 取消点赞歌曲

### 类 `MusicLibrary`

表示音乐库，管理所有的艺术家、专辑和歌曲。

#### 属性

- `artists`: 艺术家字典
- `albums`: 专辑字典
- `songs`: 歌曲字典

#### 方法

- `add_song(song)`: 添加歌曲
- `add_album(album)`: 添加专辑
- `add_artist(artist)`: 添加艺术家
- `find_artist_by_name(name)`: 通过名称查找艺术家
- `find_album_by_name(name)`: 通过名称查找专辑
- `find_song_by_name(name)`: 通过名称查找歌曲
- `goto_album(album_uuid)`: 通过 UUID 查找专辑
- `goto_artist(artist_uuid)`: 通过 UUID 查找艺术家
- `goto_song(song_uuid)`: 通过 UUID 查找歌曲
- `scan(directory)`: 扫描指定目录，提取音乐文件的信息
- `extract_id3_tags(file_path)`: 提取指定文件的 ID3 标签
- `extract_mp3_tags(file_path)`: 提取 MP3 文件的 ID3 标签
- `extract_flac_tags(file_path)`: 提取 FLAC 文件的 ID3 标签
- `parse_artists(title, album, artists, track_number=1, disc_number=1, album_artists=None)`: 解析艺术家信息
- `search_song(name)`: 搜索歌曲
- `search_album(name)`: 搜索专辑
- `search_artist(name)`: 搜索艺术家
- `show_songinfo(song)`: 显示歌曲信息
- `show_albuminfo(album)`: 显示专辑信息
- `show_artistinfo(artist)`: 显示艺术家信息
- `show_library()`: 显示整个音乐库
- `like_song(uuid)`: 点赞歌曲
- `unlike_song(uuid)`: 取消点赞歌曲
- `like_artist(uuid)`: 点赞艺术家
- `unlike_artist(uuid)`: 取消点赞艺术家
- `like_album(uuid)`: 点赞专辑
- `unlike_album(uuid)`: 取消点赞专辑
- `show_liked_songs()`: 显示点赞的歌曲
- `show_liked_artists()`: 显示点赞的艺术家
- `show_liked_albums()`: 显示点赞的专辑
- `search(query)`: 搜索音乐库

## 使用示例

```python
if __name__ == "__main__":
    library = MusicLibrary()
    library.scan('/path/to/your/music/directory')

    # 展示整个库
    library.show_library()

    # 查找并显示某个艺术家的信息
    artist = library.search_artist('artist_name')
    library.show_artistinfo(artist)

    # 查找并显示某个专辑的信息
    album = library.search_album('album_name')
    library.show_albuminfo(album)

    # 查找并显示某首歌曲的信息
    song = library.search_song('song_name')
    library.show_songinfo(song)

    # 点赞某个艺术家
    library.like_artist(artist.uuid)

    # 取消点赞某个专辑
    library.unlike_album(album.uuid)

    # 显示所有点赞的歌曲
    library.show_liked_songs()
```

## 注意事项

- 运行脚本时需要安装 `mutagen` 库：
  ```sh
  pip install mutagen
  ```
- 目录路径请替换为实际的音乐文件目录。