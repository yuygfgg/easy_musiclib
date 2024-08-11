import datetime
import logging
import os
import threading

import orjson
import json

from music_library.library import MusicLibrary
from music_library.models.album import Album
from music_library.models.artist import Artist
from music_library.models.event import Event
from music_library.models.song import Song


data_file = 'library_data.json'

def save_library(library):
    thread = threading.Thread(target=do_save_library, args=(library,))
    thread.start()

def do_save_library(library):
    def convert_datetime(obj):
        if isinstance(obj, datetime.datetime):
            return obj.isoformat()
        raise TypeError(f"Type {type(obj)} not serializable")

    def serialize_data(data):
        if isinstance(data, dict):
            return {k: serialize_data(v) for k, v in data.items()}
        elif isinstance(data, list):
            return [serialize_data(v) for v in data]
        elif isinstance(data, set):
            return list(data)
        elif isinstance(data, datetime.datetime):
            return data.isoformat()
        return data

    data = {
        'artists': {uuid: serialize_data(artist.__dict__) for uuid, artist in library.artists.items()},
        'albums': {uuid: serialize_data({
            **album.__dict__,
            'album_artists': [artist.uuid for artist in album.album_artists],
            'songs': [song.uuid for song in album.songs]
        }) for uuid, album in library.albums.items()},
        'songs': {uuid: serialize_data({
            **song.__dict__,
            'album': song.album['uuid'],
            'artists': [artist['uuid'] for artist in song.artists]
        }) for uuid, song in library.songs.items()},
        'events': {uuid: serialize_data({
            **event.__dict__,
            'albums': [album.uuid for album in event.albums]
        }) for uuid, event in library.events.items()},
    }
    try: 
        os.remove(data_file)
    except Exception as e:
        print(e)
    
    try:
        with open(data_file, 'wb') as file:
            file.write(orjson.dumps(data, default=convert_datetime))
        print("Library saved to file")
    except Exception as e:
        print(f"Failed to save library to file: {e}")

logging.basicConfig(level=logging.DEBUG, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

def load_library():
    library = MusicLibrary()

    def parse_datetime(obj, fields):
        for field in fields:
            if field in obj and obj[field] is not None:
                try:
                    obj[field] = datetime.datetime.fromisoformat(obj[field])
                except ValueError as e:
                    logger.error(f"Failed to parse datetime from value '{obj[field]}' in field '{field}': {e}")
                    obj[field] = None

    try:
        if os.path.exists(data_file):
            file_size = os.path.getsize(data_file)
            logger.debug(f"Loading library from file: {data_file} (size: {file_size} bytes)")

            with open(data_file, 'r', encoding='utf-8') as file:
                try:
                    data = json.load(file)
                except json.JSONDecodeError as e:
                    logger.error(f"JSON decode error at line {e.lineno} column {e.colno}: {e.msg}")
                    raise
                except UnicodeDecodeError as e:
                    logger.error(f"Unicode decode error: {e.reason} at position {e.start}-{e.end}")
                    raise

            library = MusicLibrary()
            logger.debug("Instantiated MusicLibrary")

            # Restore artists
            try:
                for uuid, artist_data in data['artists'].items():
                    parse_datetime(artist_data, ['liked_time'])
                    artist = Artist(artist_data['name'])
                    artist.uuid = artist_data['uuid']
                    artist.is_liked = artist_data['is_liked']
                    artist.liked_time = artist_data.get('liked_time')
                    artist.artist_art_path = artist_data['artist_art_path']
                    library.artists[uuid] = artist
                logger.debug("Restored artists")
            except Exception as e:
                logger.error(f"Failed to restore artists: {e}")
                raise

            # Restore albums
            try:
                for uuid, album_data in data['albums'].items():
                    parse_datetime(album_data, ['liked_time'])
                    album = Album(album_data['name'])
                    album.uuid = album_data['uuid']
                    album.is_liked = album_data['is_liked']
                    album.liked_time = album_data.get('liked_time')
                    album.album_art_path = album_data['album_art_path']
                    album.year = album_data['year']
                    album.event = album_data['event']  # 修改这里
                    album.album_artists = {library.artists[artist_uuid] for artist_uuid in album_data['album_artists']}
                    album.songs = []
                    library.albums[uuid] = album
                logger.debug("Restored albums")
            except Exception as e:
                logger.error(f"Failed to restore albums: {e}")
                raise

            # Restore songs
            restored_songs = 0
            try:
                for uuid, song_data in data['songs'].items():
                    restored_songs += 1
                    if (restored_songs % 250) == 0:
                        print(f"restored {restored_songs} songs")
                    parse_datetime(song_data, ['liked_time'])
                    album = library.albums[song_data['album']]
                    artists = [library.artists[artist_uuid] for artist_uuid in song_data['artists']]
                    event_data = song_data.get('event')
                    event = None
                    if event_data:
                        event = Event(event_data['name'])
                        event.uuid = event_data['uuid']
                    song = Song(
                        song_data['name'], album, artists, song_data['file_path'],
                        song_data['track_number'], song_data['disc_number'], song_data['year'], song_data.get('song_art_path') or " ",
                        event
                    )
                    song.uuid = song_data['uuid']
                    song.is_liked = song_data['is_liked']
                    song.liked_time = song_data.get('liked_time')

                    if song_data['file_path'] is None:
                        logger.warning(f"File path is None for song: {song_data['name']} (UUID: {song_data['uuid']})")

                    if song_data.get('song_art_path') is None:
                        logger.warning(f"Song art path is None for song: {song_data['name']} (UUID: {song_data['uuid']})")

                    library.songs[uuid] = song
                    album.songs.append(song)
                logger.debug("Restored songs")
            except Exception as e:
                logger.error(f"Failed to restore songs: {e}")
                raise

            # Restore events
            try:
                for uuid, event_data in data['events'].items():
                    parse_datetime(event_data, ['liked_time'])
                    event = Event(event_data['name'])
                    event.year = event_data['year']
                    event.uuid = event_data['uuid']
                    event.is_liked = event_data['is_liked']
                    event.liked_time = event_data.get('liked_time')
                    event.albums = [library.albums[album_uuid] for album_uuid in event_data['albums']]
                    library.events[uuid] = event
                logger.debug("Restored events")
            except Exception as e:
                logger.error(f"Failed to restore events: {e}")
                raise

            # Restore graph
            try:
                library.graph = library.build_graph()
                logger.debug("Restored graph")
            except Exception as e:
                logger.error(f"Failed to restore graph: {e}")
                raise

            logger.debug("Library loaded from file")
        else:
            logger.debug("No library file found, starting with an empty library")
    except Exception as e:
        logger.error(f"Failed to load library from file: {e}")
        raise
    return library