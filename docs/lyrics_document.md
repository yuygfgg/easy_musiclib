# LyricsSource Plugin Documentation

`LyricsSource` is an abstract base class designed for retrieving lyrics from various sources. Here's a guide on how to develop and use plugins.

## Directory Structure

Plugins should be placed in a directory named `lyrics`, with filenames ending in `.py`. For example:

```
lyrics/
    ├── base.py
    ├── source1.py
    └── source2.py
```

## Creating a Plugin

Each plugin needs to inherit from the `LyricsSource` class and implement the `get_aligned_lyrics` method.

### Example

```python
from base import LyricsSource

class MyLyricsSource(LyricsSource):
    def get_aligned_lyrics(self, title, artist, album, duration):
        # Implement logic to fetch lyrics
        return [
            {
                "lyrics": "Lyrics text",
                "similarity": 0.9  # Similarity score
            }
        ]
```

## Key Methods

### `load_sources(cls, directory)`

Loads all lyric source classes from the specified directory.

- **Parameters**: 
  - `directory`: Directory to load plugins from.
- **Returns**: A list of plugin instances.

### `get_lyrics(cls, title, artist, album, duration, directory="lyrics")`

Fetches lyrics from all loaded plugins.

- **Parameters**:
  - `title`: Song title.
  - `artist`: Artist name.
  - `album`: Album name.
  - `duration`: Song duration.
  - `directory`: Plugin directory, default is `lyrics`.
- **Returns**: The top nine results sorted by similarity.

## Usage Instructions

1. Create a new Python file in the `lyrics` directory.
2. Define a class in the file, inheriting from `LyricsSource`.
3. Implement the `get_aligned_lyrics` method.
4. Ensure the returned results contain `lyrics` and `similarity` fields.

## Notes

- Plugin files must end with `.py` and cannot be named `base.py`.
- The `get_aligned_lyrics` method should return a list of dictionaries, each containing `lyrics` (lyrics text) and `similarity` (similarity score) fields.