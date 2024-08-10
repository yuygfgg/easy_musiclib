# Implementing a New Audio Tag Extractor

## Overview

This document provides instructions on how to create a new audio tag extractor for different file formats. Follow these steps to ensure compatibility with the existing plugin system.

## Extractor Interface

### Abstract Layer

- **Base Class**: `TagExtractor`
  - **Method to Implement**: `extract`

### Method Signature

- **Input**: `file_path` (str) - The path to the audio file.
- **Output**: A dictionary containing:
  - `title` (str)
  - `album` (str)
  - `artists` (list of str)
  - `album_artists` (list of str) (use artists values if there's no album_artists tag, don't leave None!)
  - `track_number` (int)
  - `disc_number` (int)
  - `year` (int or None)
  - `event` (str or None)

## Steps to Implement

### 1. Create the Extractor File

- Name the file `<format>_extractor.py` in the `tag_extracter/` directory.

### 2. Define the Extractor Class

- The class should be named `<Format>Extractor`.
- Inherit from `TagExtractor`.

### 3. Implement the `extract` Method

```python
from mutagen.<format> import <Format>
from . import TagExtractor

class <Format>Extractor(TagExtractor):
    def extract(self, file_path):
        try:
            audio = <Format>(file_path)
            title = audio.get('title', ['Unknown Title'])[0]
            album = audio.get('album', ['Unknown Album'])[0]
            artists = audio.get('artist', ['Unknown Artist'])
            if isinstance(artists, str):
                artists = [artists]

            track_number = int(audio.get('tracknumber', ['1'])[0])
            disc_number = int(audio.get('discnumber', ['1'])[0])
            year = audio.get('date', [None])[0] or audio.get('year', [None])[0]
            year = utils.extract_year(year)
            event = audio.get('event', ['Unknown Event'])[0]

            return {
                'title': title,
                'album': album,
                'artists': artists,
                'track_number': track_number,
                'disc_number': disc_number,
                'year': year,
                'event': event
            }

        except Exception as e:
            print(f"Error reading <FORMAT> tags from {file_path}: {e}")
            return {
                'title': 'Unknown Title',
                'album': 'Unknown Album',
                'artists': ['Unknown Artist'],
                'track_number': 1,
                'disc_number': 1,
                'year': None,
                'event': 'Unknown Event'
            }
```
