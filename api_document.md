# API 文档

这是一个基于 Flask 的音乐库 API 文档。通过这些 API 端点，您可以管理音乐库中的歌曲、专辑和艺术家，并执行各种操作如添加、搜索、喜欢等。

## 基础信息

- **基础 URL**: `/`
- **响应格式**: JSON
- **端口**: `5010`

## 端点

### 添加歌曲

- **URL**: `/add_song`
- **方法**: `GET`
- **参数**:
  - `name` (string): 歌曲名称
  - `album_uuid` (string): 专辑的 UUID
  - `artist_uuids` (list of string): 艺术家的 UUID 列表
  - `file_path` (string): 歌曲文件路径
  - `track_number` (int, optional): 轨道号，默认为 1
  - `disc_number` (int, optional): 光盘号，默认为 1
  - `year` (string, optional): 发行年份
- **响应**: 
  - `201 Created` 成功添加歌曲
  - `400 Bad Request` 参数缺失或错误
- **示例**:
  ```json
  {
    "message": "Song added",
    "uuid": "abc123",
    "year": "2020"
  }
  ```

### 添加专辑

- **URL**: `/add_album`
- **方法**: `GET`
- **参数**:
  - `name` (string): 专辑名称
  - `year` (string, optional): 发行年份
- **响应**: 
  - `201 Created` 成功添加专辑
  - `400 Bad Request` 参数缺失或错误
- **示例**:
  ```json
  {
    "message": "Album added",
    "uuid": "def456",
    "year": "2020"
  }
  ```

### 添加艺术家

- **URL**: `/add_artist`
- **方法**: `GET`
- **参数**:
  - `name` (string): 艺术家名称
- **响应**: 
  - `201 Created` 成功添加艺术家
  - `400 Bad Request` 参数缺失或错误
- **示例**:
  ```json
  {
    "message": "Artist added",
    "uuid": "ghi789"
  }
  ```

### 喜欢歌曲

- **URL**: `/like_song/<uuid>`
- **方法**: `GET`
- **参数**: 
  - `uuid` (string): 歌曲的 UUID
- **响应**: 
  - `200 OK` 成功喜欢歌曲
  - `404 Not Found` 未找到歌曲
- **示例**:
  ```json
  {
    "message": "Song liked"
  }
  ```

### 取消喜欢歌曲

- **URL**: `/unlike_song/<uuid>`
- **方法**: `GET`
- **参数**: 
  - `uuid` (string): 歌曲的 UUID
- **响应**: 
  - `200 OK` 成功取消喜欢歌曲
  - `404 Not Found` 未找到歌曲
- **示例**:
  ```json
  {
    "message": "Song unliked"
  }
  ```

### 喜欢专辑

- **URL**: `/like_album/<uuid>`
- **方法**: `GET`
- **参数**: 
  - `uuid` (string): 专辑的 UUID
- **响应**: 
  - `200 OK` 成功喜欢专辑
  - `404 Not Found` 未找到专辑
- **示例**:
  ```json
  {
    "message": "Album liked"
  }
  ```

### 取消喜欢专辑

- **URL**: `/unlike_album/<uuid>`
- **方法**: `GET`
- **参数**: 
  - `uuid` (string): 专辑的 UUID
- **响应**: 
  - `200 OK` 成功取消喜欢专辑
  - `404 Not Found` 未找到专辑
- **示例**:
  ```json
  {
    "message": "Album unliked"
  }
  ```

### 喜欢艺术家

- **URL**: `/like_artist/<uuid>`
- **方法**: `GET`
- **参数**: 
  - `uuid` (string): 艺术家的 UUID
- **响应**: 
  - `200 OK` 成功喜欢艺术家
  - `404 Not Found` 未找到艺术家
- **示例**:
  ```json
  {
    "message": "Artist liked"
  }
  ```

### 取消喜欢艺术家

- **URL**: `/unlike_artist/<uuid>`
- **方法**: `GET`
- **参数**: 
  - `uuid` (string): 艺术家的 UUID
- **响应**: 
  - `200 OK` 成功取消喜欢艺术家
  - `404 Not Found` 未找到艺术家
- **示例**:
  ```json
  {
    "message": "Artist unliked"
  }
  ```

### 扫描目录

- **URL**: `/scan`
- **方法**: `GET`
- **参数**:
  - `directory` (string): 要扫描的目录路径
- **响应**: 
  - `200 OK` 扫描完成
  - `400 Bad Request` 参数缺失或错误
- **示例**:
  ```json
  {
    "message": "Scan completed"
  }
  ```

### 搜索歌曲

- **URL**: `/search_song/<name>`
- **方法**: `GET`
- **参数**: 
  - `name` (string): 歌曲名称
- **响应**: 
  - `200 OK` 找到歌曲
  - `404 Not Found` 未找到歌曲
- **示例**:
  ```json
  {
    "name": "Song Name",
    "uuid": "abc123"
  }
  ```

### 搜索专辑

- **URL**: `/search_album/<name>`
- **方法**: `GET`
- **参数**: 
  - `name` (string): 专辑名称
- **响应**: 
  - `200 OK` 找到专辑
  - `404 Not Found` 未找到专辑
- **示例**:
  ```json
  {
    "name": "Album Name",
    "uuid": "def456"
  }
  ```

### 搜索艺术家

- **URL**: `/search_artist/<name>`
- **方法**: `GET`
- **参数**: 
  - `name` (string): 艺术家名称
- **响应**: 
  - `200 OK` 找到艺术家
  - `404 Not Found` 未找到艺术家
- **示例**:
  ```json
  {
    "name": "Artist Name",
    "uuid": "ghi789"
  }
  ```

### 显示音乐库

- **URL**: `/show_library`
- **方法**: `GET`
- **响应**: 
  - `200 OK` 返回音乐库信息
- **示例**:
  ```json
  {
    "songs": [...],
    "albums": [...],
    "artists": [...]
  }
  ```

### 显示喜欢的歌曲

- **URL**: `/show_liked_songs`
- **方法**: `GET`
- **响应**: 
  - `200 OK` 返回喜欢的歌曲列表
- **示例**:
  ```json
  [
    {
      "name": "Song Name",
      "uuid": "abc123",
      ...
    }
  ]
  ```

### 显示喜欢的艺术家

- **URL**: `/show_liked_artists`
- **方法**: `GET`
- **响应**: 
  - `200 OK` 返回喜欢的艺术家列表
- **示例**:
  ```json
  [
    {
      "name": "Artist Name",
      "uuid": "ghi789",
      ...
    }
  ]
  ```

### 显示喜欢的专辑

- **URL**: `/show_liked_albums`
- **方法**: `GET`
- **响应**: 
  - `200 OK` 返回喜欢的专辑列表
- **示例**:
  ```json
  [
    {
      "name": "Album Name",
      "uuid": "def456",
      ...
    }
  ]
  ```

### 显示歌曲详情

- **URL**: `/show_song/<uuid>`
- **方法**: `GET`
- **参数**: 
  - `uuid` (string): 歌曲的 UUID
- **响应**: 
  - `200 OK` 返回歌曲详情
  - `404 Not Found` 未找到歌曲
- **示例**:
  ```json
  {
    "name": "Song Name",
    "uuid": "abc123",
    ...
  }
  ```

### 显示专辑详情

- **URL**: `/show_album/<uuid>`
- **方法**: `GET`
- **参数**: 
  - `uuid` (string): 专辑的 UUID
- **响应**: 
  - `200 OK` 返回专辑详情
  - `404 Not Found` 未找到专辑
- **示例**:
  ```json
  {
    "name": "Album Name",
    "uuid": "def456",
    ...
  }
  ```

### 显示艺术家详情

- **URL**: `/show_artist/<uuid>`
- **方法**: `GET`
- **参数**: 
  - `uuid` (string): 艺术家的 UUID
- **响应**: 
  - `200 OK` 返回艺术家详情
  - `404 Not Found` 未找到艺术家
- **示例**:
  ```json
  {
    "name": "Artist Name",
    "uuid": "ghi789",
    ...
  }
  ```

### 搜索

- **URL**: `/search/<query>`
- **方法**: `GET`
- **参数**: 
  - `query` (string): 搜索关键词
- **响应**: 
  - `200 OK` 返回搜索结果
- **示例**:
  ```json
  {
    "songs": [...],
    "albums": [...],
    "artists": [...]
  }
  ```

### 获取文件

- **URL**: `/getfile`
- **方法**: `GET`
- **参数**: 
  - `file_path` (string): 文件路径
- **响应**: 
  - `200 OK` 返回文件
  - `404 Not Found` 未找到文件
- **示例**:
  ```json
  // 文件流
  ```

### 获取音频流

- **URL**: `/getStream`
- **方法**: `GET`
- **参数**: 
  - `file_path` (string): 音频文件路径
- **响应**: 
  - `200 OK` 返回音频流
  - `400 Bad Request` 不支持的音频格式
  - `404 Not Found` 未找到文件
- **示例**:
  ```json
  // 音频流
  ```

### 显示关系

- **URL**: `/show_relation`
- **方法**: `GET`
- **参数**: 
  - `uuid` (string): 艺术家的 UUID
  - `layer` (int): 显示关系的层级
- **响应**: 
  - `200 OK` 返回关系树
  - `400 Bad Request` 参数缺失或错误
- **示例**:
  ```json
  {
    // 关系树结构
  }
  ```

### 合并艺术家（通过 UUID）

- **URL**: `/merge_artist_by_uuid`
- **方法**: `GET`
- **参数**: 
  - `uuid1` (string): 第一个艺术家的 UUID
  - `uuid2` (string): 第二个艺术家的 UUID
- **响应**: 
  - `200 OK` 合并成功
  - `400 Bad Request` 参数缺失或错误
- **示例**:
  ```json
  {
    "message": "Artist uuid2 merged into uuid1"
  }
  ```

### 合并艺术家（通过名称）

- **URL**: `/merge_artist_by_name`
- **方法**: `GET`
- **参数**: 
  - `name1` (string): 第一个艺术家的名称
  - `name2` (string): 第二个艺术家的名称
- **响应**: 
  - `200 OK` 合并成功
  - `400 Bad Request` 参数缺失或错误
- **示例**:
  ```json
  {
    "message": "Artist name2 merged into name1"
  }
  ```

### 自动合并

- **URL**: `/auto_merge`
- **方法**: `GET`
- **响应**: 
  - `200 OK` 自动合并完成
- **示例**:
  ```json
  {
    "message": "Auto merge completed"
  }
  ```

### 获取歌词

- **URL**: `/lyrics`
- **方法**: `GET`
- **参数**: 
  - `title` (string): 歌曲标题
  - `artist` (string, optional): 艺术家名称
  - `album` (string, optional): 专辑名称
  - `duration` (float, optional): 歌曲时长
- **响应**: 
  - `200 OK` 返回歌词
  - `404 Not Found` 未找到歌词
- **示例**:
  ```json
  [
    {
      "title": "Song Title",
      "artist": "Artist Name",
      "lyrics": "Lyrics content"
    }
  ]
  ```

## 错误处理

所有的错误响应将包含一个 `message` 字段，描述错误的原因。例如：

```json
{
  "message": "File not found"
}
```

