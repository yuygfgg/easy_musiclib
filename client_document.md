# MusicLibraryClient 和 MusicLibraryConsole 使用文档

## 概述

`client.py` 提供了一个音乐库客户端和控制台程序，用于与一个基于 REST API 的音乐库服务进行交互。它允许用户添加、搜索、喜欢和取消喜欢歌曲、专辑和艺术家，还可以播放歌曲并显示音乐库的详细信息。

## 类定义

### `MusicLibraryClient`

`MusicLibraryClient` 类用于与音乐库 REST API 进行通信。它提供了一系列方法来执行各种操作。

#### 初始化

```python
def __init__(self, base_url="http://127.0.0.1:5000"):
```

初始化客户端，设置基础 URL 和请求头。

#### 方法

- `add_song(name, album_uuid, artist_uuids, file_path, track_number=1, disc_number=1)`
- `add_album(name)`
- `add_artist(name)`
- `search_song(name)`
- `search_album(name)`
- `search_artist(name)`
- `like_song(uuid)`
- `unlike_song(uuid)`
- `like_album(uuid)`
- `unlike_album(uuid)`
- `like_artist(uuid)`
- `unlike_artist(uuid)`
- `show_library()`
- `show_liked_songs()`
- `show_liked_artists()`
- `show_liked_albums()`
- `scan(directory)`
- `show_song(uuid)`
- `show_album(uuid)`
- `show_artist(uuid)`
- `search(query)`
- `get_file(file_path)`

每个方法都会向定义的 API 端点发送 HTTP 请求，并返回响应的 JSON 数据。

### `MusicLibraryConsole`

`MusicLibraryConsole` 类继承自 `Cmd`，提供一个交互式命令行界面，允许用户与 `MusicLibraryClient` 进行交互。

#### 方法

- `pretty_print_json(data)`: 格式化并打印 JSON 数据。
- `print_song_details(song_info)`: 打印歌曲详细信息。
- `print_album_details(album_info)`: 打印专辑详细信息。
- `print_artist_details(artist_info)`: 打印艺术家详细信息。
- `print_library_details(library_info)`: 打印音乐库详细信息。
- `print_search_results(search_results)`: 打印搜索结果。
- `print_liked_songs(liked_songs)`: 打印喜欢的歌曲。
- `print_liked_artists(liked_artists)`: 打印喜欢的艺术家。
- `print_liked_albums(liked_albums)`: 打印喜欢的专辑。

#### 命令

- `search_album <name>`: 搜索专辑。
- `search_artist <name>`: 搜索艺术家。
- `search_song <name>`: 搜索歌曲。
- `show_library`: 显示音乐库。
- `show_album <uuid>`: 显示专辑详情。
- `show_artist <uuid>`: 显示艺术家详情。
- `show_song <uuid>`: 显示歌曲详情。
- `show_liked_songs`: 显示喜欢的歌曲。
- `show_liked_artists`: 显示喜欢的艺术家。
- `show_liked_albums`: 显示喜欢的专辑。
- `play_song <uuid>`: 播放歌曲。
- `search <query>`: 模糊搜索。
- `like_song <uuid>`: 喜欢歌曲。
- `unlike_song <uuid>`: 取消喜欢歌曲。
- `like_album <uuid>`: 喜欢专辑。
- `unlike_album <uuid>`: 取消喜欢专辑。
- `like_artist <uuid>`: 喜欢艺术家。
- `unlike_artist <uuid>`: 取消喜欢艺术家。
- `exit`: 退出控制台。

## 使用示例

### 启动控制台

```bash
python client.py
```

### 添加歌曲

```plain
MusicLibrary> add_song "Song Name" "Album UUID" ["Artist UUID1", "Artist UUID2"] "file/path.mp3"
```

### 搜索专辑

```plain
MusicLibrary> search_album "Album Name"
```

### 播放歌曲

```plain
MusicLibrary> play_song "Song UUID"
```

### 退出控制台

```plain
MusicLibrary> exit
```

## 依赖

确保安装以下 Python 包：

- requests
- pygments

可使用 pip 安装：

```bash
pip install requests pygments
```

## 注意

- 确保音乐库服务正在运行，并且 `base_url` 正确指向服务地址。
- 播放歌曲功能依赖 `mpv` 播放器，确保本地已安装 `mpv`。