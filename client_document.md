# MusicLibraryClient Command Guide

This guide will help you use the `MusicLibraryClient` and `MusicLibraryConsole` to effectively manage and interact with your music library.

## Installation

Before starting, ensure you have the necessary dependencies installed. You can install them using the following command:

```bash
pip install requests pygments
```

## Classes and Methods Overview

### MusicLibraryClient Class

`MusicLibraryClient` is a client class for interacting with the music library service. It provides a series of methods to add, search, like, and display contents of the music library.

#### Initialization

```python
client = MusicLibraryClient(base_url="http://127.0.0.1:5000")
```

You can specify a custom `base_url`, default is `http://127.0.0.1:5000`.

#### Adding

- **Add Song**

  ```python
  client.add_song(name, album_uuid, artist_uuids, file_path, track_number=1, disc_number=1, year=None)
  ```

- **Add Album**

  ```python
  client.add_album(name, year=None)
  ```

- **Add Artist**

  ```python
  client.add_artist(name)
  ```

#### Searching

- **Search Song**

  ```python
  client.search_song(name)
  ```

- **Search Album**

  ```python
  client.search_album(name)
  ```

- **Search Artist**

  ```python
  client.search_artist(name)
  ```

#### Liking/Unliking

- **Like/Unlike Song**

  ```python
  client.like_song(uuid)
  client.unlike_song(uuid)
  ```

- **Like/Unlike Album**

  ```python
  client.like_album(uuid)
  client.unlike_album(uuid)
  ```

- **Like/Unlike Artist**

  ```python
  client.like_artist(uuid)
  client.unlike_artist(uuid)
  ```

#### Displaying

- **Show Entire Music Library**

  ```python
  client.show_library()
  ```

- **Show Liked Songs**

  ```python
  client.show_liked_songs()
  ```

- **Show Liked Artists**

  ```python
  client.show_liked_artists()
  ```

- **Show Liked Albums**

  ```python
  client.show_liked_albums()
  ```

- **Scan Directory**

  ```python
  client.scan(directory)
  ```

- **Show Specific Song, Album, or Artist Details**

  ```python
  client.show_song(uuid)
  client.show_album(uuid)
  client.show_artist(uuid)
  ```

#### Get File

```python
client.get_file(file_path)
```

### MusicLibraryConsole Class

`MusicLibraryConsole` provides a command-line interface for interacting with `MusicLibraryClient`.

#### Starting the Console

```python
if __name__ == "__main__":
    console = MusicLibraryConsole()
    console.cmdloop()
```

#### Available Commands

- **Search**

  ```plaintext
  search_album <name>
  search_artist <name>
  search_song <name>
  search <query>  # Fuzzy search
  ```

- **Display**

  ```plaintext
  show_library
  show_album <uuid>
  show_artist <uuid>
  show_song <uuid>
  show_liked_songs
  show_liked_artists
  show_liked_albums
  ```

- **Like/Unlike**

  ```plaintext
  like_song <uuid>
  unlike_song <uuid>
  like_album <uuid>
  unlike_album <uuid>
  like_artist <uuid>
  unlike_artist <uuid>
  ```

- **Play Song**

  ```plaintext
  play_song <uuid>
  ```

- **Scan Directory**

  ```plaintext
  scan_directory <directory>
  ```

- **Exit Console**

  ```plaintext
  exit
  ```

## Examples

Here are some examples of how to use `MusicLibraryConsole`:

1. **Search for an Album**

    ```plaintext
    MusicLibrary> search_album MyAlbum
    ```

2. **Show Entire Music Library**

    ```plaintext
    MusicLibrary> show_library
    ```

3. **Like a Song**

    ```plaintext
    MusicLibrary> like_song 123e4567-e89b-12d3-a456-426614174000
    ```

4. **Play a Song**

    ```plaintext
    MusicLibrary> play_song 123e4567-e89b-12d3-a456-426614174000
    ```

By following this guide, you should be able to effectively use `MusicLibraryClient` and `MusicLibraryConsole` to manage your music library. If you have any questions, refer to the comments in the code or use the `help` command in the console for more information.