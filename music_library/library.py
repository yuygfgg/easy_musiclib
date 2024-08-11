import importlib
import os
from functools import lru_cache

from music_library import utils


from .tag_extracter import TagExtractor
from .display import MusicLibraryDisplay
from .search import MusicLibrarySearch
from .graph import MusicLibraryGraph
from .merge_artists import MusicLibraryMergeArtist
from .scan import MusicLibraryScan


class MusicLibrary(
    MusicLibrarySearch, MusicLibraryGraph, MusicLibraryMergeArtist, MusicLibraryScan
):
    def __init__(self):
        self.events = {}
        self.artists = {}
        self.albums = {}
        self.songs = {}
        self.display = MusicLibraryDisplay(self)
        self.graph = {}
        self.extractors = self.load_extractors()

    def __getstate__(self):
        # 获取对象的状态，并移除不可序列化的部分
        state = self.__dict__.copy()
        return state

    def __setstate__(self, state):
        self.__dict__.update(state)

    def load_extractors(self):
        extractors = {}
        for file in os.listdir("music_library/tag_extracter"):
            if file.endswith("_extractor.py"):
                module_name = file[:-3]
                module = importlib.import_module(
                    f"music_library.tag_extracter.{module_name}"
                )
                for attr in dir(module):
                    cls = getattr(module, attr)
                    if (
                        isinstance(cls, type)
                        and issubclass(cls, TagExtractor)
                        and cls is not TagExtractor
                    ):
                        # Extract the extension from the module name
                        extension = module_name.split("_")[0]
                        extractors[extension] = cls()
        return extractors

    def extract_tags(self, file_path):
        extension = os.path.splitext(file_path)[1][1:].lower()
        extractor = self.extractors.get(extension)
        if extractor:
            return extractor.extract(file_path)
        return None

    def add_song(self, song):
        self.songs[song.uuid] = song

    def add_album(self, album):
        self.albums[album.uuid] = album

    def add_artist(self, artist):
        self.artists[artist.uuid] = artist

    def add_event(self, event):
        self.events[event.uuid] = event

    @lru_cache(maxsize=None)
    def find_event_by_name(self, name):
        normalized_name = utils.normalize_name(name)
        for event in self.events.values():
            if utils.normalize_name(event.name) == normalized_name:
                return event
        return None

    @lru_cache(maxsize=None)
    def find_artist_by_name(self, name):
        normalized_name = utils.normalize_name(name)
        for artist in self.artists.values():
            if utils.normalize_name(artist.name) == normalized_name:
                return artist
        return None

    def find_album_by_name(self, name):
        name = name.strip()
        for album in self.albums.values():
            if album.name == name:
                return album
        return None

    def find_song_by_name(self, name):
        name = name.strip()
        for song in self.songs.values():
            if song.name == name:
                return song
        return None

    def goto_album(self, album_uuid):
        return self.albums.get(album_uuid, None)

    def goto_artist(self, artist_uuid):
        return self.artists.get(artist_uuid, None)

    def goto_song(self, song_uuid):
        return self.songs.get(song_uuid, None)

    def like_song(self, uuid):
        song = self.songs.get(uuid)
        if song:
            song.like()
        else:
            print(f"Song {uuid} not found.")

    def unlike_song(self, uuid):
        song = self.songs.get(uuid)
        if song:
            song.unlike()
        else:
            print(f"Song {uuid} not found.")

    def like_artist(self, uuid):
        artist = self.artists.get(uuid)
        if artist:
            artist.like()
        else:
            print(f"Artist {uuid} not found.")

    def unlike_artist(self, uuid):
        artist = self.artists.get(uuid)
        if artist:
            artist.unlike()
        else:
            print(f"Artist {uuid} not found.")

    def like_album(self, uuid):
        album = self.albums.get(uuid)
        if album:
            album.like()
        else:
            print(f"Album {uuid} not found.")

    def unlike_album(self, uuid):
        album = self.albums.get(uuid)
        if album:
            album.unlike()
        else:
            print(f"Album {uuid} not found.")


if __name__ == "__main__":
    library = MusicLibrary()
    library.scan("/Users/a1/other")

    library.display.show_library()
