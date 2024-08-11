from functools import lru_cache
import os

from . import utils

from .models import Event, Album, Artist, Song


class MusicLibraryScan:
    @lru_cache(maxsize=None)
    def find_album_by_name_artist_year(
        self, name, album_artist_names, year, album_artist_tag
    ):
        album_artist_names_set = {
            utils.normalize_name(album_artist_name)
            for album_artist_name in album_artist_names
        }
        if not album_artist_tag:
            album_artist_names = None
            album_artist_names_set = None
        for album in self.albums.values():
            album_artist_names_set_in_album = {
                utils.normalize_name(artist.name) for artist in album.album_artists
            }
            if (
                (
                    utils.normalize_name(album.name) == utils.normalize_name(name)
                    or album.name is None
                    or name is None
                )
                and (album.year == year or album.year is None or year is None)
                and (
                    album_artist_names_set == album_artist_names_set_in_album
                    or not album_artist_names
                    or not album.album_artists
                )
            ):
                return album
        return None

    def scan(self, directory):
        scanned_count = 0
        existing_files = {song.file_path for song in self.songs.values()}

        for root, dirs, files in os.walk(directory):
            for file in files:
                file_path = os.path.join(root, file)
                if file_path not in existing_files:
                    scanned_count += 1
                    if scanned_count % 250 == 0:
                        print(f"Scanned {scanned_count} files.")
                    self.scan_file(file_path)

        self.graph = self.build_graph()
        self.auto_merge()
        self.search.cache_clear()

    @lru_cache(maxsize=None)
    def scan_file(self, file_path):
        try:
            album_artist_tag = True
            tags = self.extract_tags(file_path)
            if not tags:
                return

            song_name = tags["title"].strip()
            album_name = tags["album"].strip()
            artist_names = utils.parse_artists(tags["artists"])
            album_artist_names = utils.parse_artists(
                tags.get("album_artists", artist_names)
            )
            track_number = tags["track_number"]
            disc_number = tags["disc_number"]
            year = tags["year"]
            event_name = (
                tags["event"][0].strip()
                if isinstance(tags["event"], list)
                else tags["event"].strip()
            )

            if album_artist_names == artist_names:
                album_artist_tag = False

            # Handle album and artists
            album = self.find_or_create_album(
                album_name, album_artist_names, year, album_artist_tag
            )
            song_artists = self.find_or_create_artists(artist_names)

            # Handle event
            event = self.find_or_create_event(event_name, album)

            song_art_path = album.album_art_path if album.album_art_path else None

            song = Song(
                song_name,
                album,
                song_artists,
                file_path,
                track_number,
                disc_number,
                year,
                song_art_path,
                event,
            )
            self.add_song(song)
            album.songs.append(song)
            album.update_year()
            album.update_event()
            if event:
                event.update_year()

            self.update_art_paths(album, song, song_artists)

        except Exception as e:
            print(f"Error processing file {file_path}: {e}")
            raise

    def find_or_create_album(
        self, album_name, album_artist_names, year, album_artist_tag
    ):
        album = self.find_album_by_name_artist_year(
            album_name, tuple(album_artist_names), year, album_artist_tag
        )
        if not album:
            print(f"Adding new album {album_name}.")
            album = Album(album_name)
            self.find_album_by_name_artist_year.cache_clear()
            album.year = year
            for artist_name in album_artist_names:
                artist = self.find_artist_by_name(artist_name)
                if not artist:
                    print(f"Adding new artist {artist_name}.")
                    artist = Artist(artist_name)
                    self.find_artist_by_name.cache_clear()
                    self.add_artist(artist)
                album.album_artists.add(artist)
            self.add_album(album)
        return album

    def find_or_create_artists(self, artist_names):
        artists = []
        for artist_name in artist_names:
            artist = self.find_artist_by_name(artist_name)
            if not artist:
                artist = Artist(artist_name)
                self.find_artist_by_name.cache_clear()
                self.add_artist(artist)
            artists.append(artist)
        return artists

    def find_or_create_event(self, event_name, album):
        if not event_name:
            return None
        event = self.find_event_by_name(event_name)
        if not event:
            print(f"Adding new event {event_name}.")
            event = Event(event_name)
            self.find_event_by_name.cache_clear()
            self.add_event(event)
        if album not in event.albums:
            event.albums.append(album)
        if not album.event or not album.event["uuid"]:
            album.event = {"uuid": event.uuid, "name": event.name}
        return event

    def update_art_paths(self, album, song, song_artists):
        if not album.album_art_path:
            album.album_art_path = song.song_art_path
            if not album.album_art_path:
                album.album_art_path = self.extract_embedded_art(song.file_path)
                if album.album_art_path:
                    for s in album.songs:
                        if not s.song_art_path:
                            s.song_art_path = album.album_art_path

        for artist in album.album_artists:
            if not artist.artist_art_path:
                artist.artist_art_path = album.album_art_path

        for artist in song_artists:
            if not artist.artist_art_path:
                artist.artist_art_path = album.album_art_path
                if not artist.artist_art_path:
                    artist.artist_art_path = song.song_art_path
