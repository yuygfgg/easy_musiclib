from mutagen.flac import FLAC
from music_library import utils
from . import TagExtractor


class FlacExtractor(TagExtractor):
    def extract(self, file_path):
        try:
            audio = FLAC(file_path)
            title = audio.get("title", ["Unknown Title"])[0]
            album = audio.get("album", ["Unknown Album"])[0]
            artists = audio.get("artist", ["Unknown Artist"])
            event = audio.get("event", ["Unknown Event"])
            if isinstance(artists, str):
                artists = [artists]

            album_artists = audio.get("albumartist")
            if not album_artists:
                album_artists = audio.get("album artist", artists)
            if isinstance(album_artists, str):
                album_artists = [album_artists]

            try:
                track_number = int(audio.get('tracknumber', ['1'])[0].split('/')[0])
            except (ValueError, IndexError) as e:
                print(f"Error processing track number: {e}")
                track_number = 1

            try:
                disc_number = int(audio.get('discnumber', ['1'])[0].split('/')[0])
            except (ValueError, IndexError) as e:
                print(f"Error processing disc number: {e}")
                disc_number = 1
            
            year = audio.get("date", [None])[0] or audio.get("year", [None])[0]
            year = utils.extract_year(year)
            date = audio.get("date", [year])[0]
            return {
                "title": title,
                "album": album,
                "artists": artists,
                "album_artists": album_artists,
                "track_number": track_number,
                "disc_number": disc_number,
                "date": date,
                "year": year,
                "event": event,
            }

        except Exception as e:
            print(f"Error reading FLAC tags from {file_path}: {e}")
            return {
                "title": "Unknown Title",
                "album": "Unknown Album",
                "artists": ["Unknown Artist"],
                "album_artists": ["Unknown Artist"],
                "track_number": 1,
                "disc_number": 1,
                "event": "Unknown Event",
                "date": None,
                "year": None,
            }

