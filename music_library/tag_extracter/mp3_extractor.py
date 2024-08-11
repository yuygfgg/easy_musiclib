from mutagen.easyid3 import EasyID3

from music_library import utils
from . import TagExtractor


class Mp3Extractor(TagExtractor):
    def extract(self, file_path):
        try:
            audio = EasyID3(file_path)
            title = audio.get("title", ["Unknown Title"])[0]
            album = audio.get("album", ["Unknown Album"])[0]
            artists = audio.get("artist", ["Unknown Artist"])
            event = audio.get("event", ["Unknown Event"])
            if isinstance(artists, str):
                artists = [artists]

            album_artists = audio.get("albumartist", artists)
            if isinstance(album_artists, str):
                album_artists = [album_artists]

            track_number = int(audio.get("tracknumber", ["1"])[0].split("/")[0])
            disc_number = int(audio.get("discnumber", ["1"])[0].split("/")[0])
            date = audio.get("date", [None])[0]
            year = utils.extract_year(date)

            return {
                "title": title,
                "album": album,
                "artists": artists,
                "album_artists": album_artists,
                "track_number": track_number,
                "disc_number": disc_number,
                "year": year,
                "date": date,
                "event": event,
            }

        except Exception as e:
            print(f"Error reading MP3 tags from {file_path}: {e}")
            return {
                "title": "Unknown Title",
                "album": "Unknown Album",
                "artists": ["Unknown Artist"],
                "album_artists": ["Unknown Artist"],
                "track_number": 1,
                "disc_number": 1,
                "year": None,
                "date": None,
                "event": "Unknown Event",
            }
