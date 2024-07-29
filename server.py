import threading
from flask import Flask, Response, request, jsonify, send_file, send_from_directory
from musiclib import MusicLibrary, Artist, Album, Song
import os
import sys
from flask_cors import CORS
from flask_compress import Compress
import json
import orjson
import opencc
import datetime
import logging
import lrc

app = Flask(__name__)
CORS(app)
Compress(app)

library = MusicLibrary()
data_file = 'library_data.json'
current_dir = os.path.dirname(os.path.abspath(__file__))
static_folder = os.path.join(current_dir, 'webui')

def save_library():
    thread = threading.Thread(target=do_save_library)
    thread.start()

def do_save_library():
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
    global library

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
                    album.event = album_data['event']
                    album.album_artists = {library.artists[artist_uuid] for artist_uuid in album_data['album_artists']}
                    album.songs = []
                    library.albums[uuid] = album
                logger.debug("Restored albums")
            except Exception as e:
                logger.error(f"Failed to restore albums: {e}")
                raise
            restored_songs = 0
            # Restore songs
            try:
                for uuid, song_data in data['songs'].items():
                    restored_songs += 1
                    if (restored_songs % 250) == 0:
                        print(f"restored {restored_songs} songs")
                    parse_datetime(song_data, ['liked_time'])
                    album = library.albums[song_data['album']]
                    artists = [library.artists[artist_uuid] for artist_uuid in song_data['artists']]
                    song = Song(
                        song_data['name'], album, artists, song_data['file_path'],
                        song_data['track_number'], song_data['disc_number'], song_data['year'], song_data.get('song_art_path'), song_data.get('event')
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

            # Restore graph
            try:
                # library.graph = {k: {kk: {'strength': vv['strength'], 'details': set(vv['details'])} for kk, vv in v.items()} for k, v in data.get('graph', {}).items()}
                library.graph = library.build_graph()
                logger.debug("Restored graph")
            except Exception as e:
                logger.error(f"Failed to restore graph: {e}")
                raise

            # Reinitialize OpenCC object
            try:
                library.cc = opencc.OpenCC('t2s')
                logger.debug("Reinitialized OpenCC object")
            except Exception as e:
                logger.error(f"Failed to reinitialize OpenCC object: {e}")
                raise

            logger.debug("Library loaded from file")
        else:
            logger.debug("No library file found, starting with an empty library")
    except Exception as e:
        logger.error(f"Failed to load library from file: {e}")
        raise


@app.route('/api/add_song', methods=['GET'])
def add_song():
    name = request.args.get('name')
    album_uuid = request.args.get('album_uuid')
    artist_uuids = request.args.getlist('artist_uuids')
    file_path = request.args.get('file_path')
    track_number = int(request.args.get('track_number', 1))
    disc_number = int(request.args.get('disc_number', 1))
    year = request.args.get('year')

    album = library.goto_album(album_uuid)
    artists = [library.goto_artist(uuid) for uuid in artist_uuids]

    song = Song(name, album, artists, file_path, track_number, disc_number, year)
    library.add_song(song)
    album.update_year()
    save_library()
    return jsonify({'message': 'Song added', 'uuid': song.uuid, 'year': song.year}), 201

@app.route('/api/add_album', methods=['GET'])
def add_album():
    name = request.args.get('name')
    year = request.args.get('year')
    album = Album(name)
    album.year = year
    library.add_album(album)
    save_library()
    return jsonify({'message': 'Album added', 'uuid': album.uuid, 'year': album.year}), 201

@app.route('/api/add_artist', methods=['GET'])
def add_artist():
    name = request.args.get('name')
    artist = Artist(name)
    library.add_artist(artist)
    save_library()
    return jsonify({'message': 'Artist added', 'uuid': artist.uuid}), 201

@app.route('/api/like_song/<uuid>', methods=['GET'])
def like_song(uuid):
    library.like_song(uuid)
    save_library()
    return jsonify({'message': 'Song liked'}), 200

@app.route('/api/unlike_song/<uuid>', methods=['GET'])
def unlike_song(uuid):
    library.unlike_song(uuid)
    save_library()
    return jsonify({'message': 'Song unliked'}), 200

@app.route('/api/like_album/<uuid>', methods=['GET'])
def like_album(uuid):
    library.like_album(uuid)
    save_library()
    return jsonify({'message': 'Album liked'}), 200

@app.route('/api/unlike_album/<uuid>', methods=['GET'])
def unlike_album(uuid):
    library.unlike_album(uuid)
    save_library()
    return jsonify({'message': 'Album unliked'}), 200

@app.route('/api/like_artist/<uuid>', methods=['GET'])
def like_artist(uuid):
    library.like_artist(uuid)
    save_library()
    return jsonify({'message': 'Artist liked'}), 200

@app.route('/api/unlike_artist/<uuid>', methods=['GET'])
def unlike_artist(uuid):
    library.unlike_artist(uuid)
    save_library()
    return jsonify({'message': 'Artist unliked'}), 200

@app.route('/api/scan', methods=['GET'])
def scan():
    directory = request.args.get('directory')
    if not directory:
        return jsonify({'message': 'Directory is required'}), 400

    library.scan(directory)
    save_library()
    return jsonify({'message': 'Scan completed'}), 200

@app.route('/api/search_song/<name>', methods=['GET'])
def search_song(name):
    song = library.search_song(name)
    if song:
        return jsonify({'name': song.name, 'uuid': song.uuid}), 200
    return jsonify({'message': 'Song not found'}), 404

@app.route('/api/search_album/<name>', methods=['GET'])
def search_album(name):
    album = library.search_album(name)
    if album:
        return jsonify({'name': album.name, 'uuid': album.uuid}), 200
    return jsonify({'message': 'Album not found'}), 404

@app.route('/api/search_artist/<name>', methods=['GET'])
def search_artist(name):
    artist = library.search_artist(name)
    if artist:
        return jsonify({'name': artist.name, 'uuid': artist.uuid}), 200
    return jsonify({'message': 'Artist not found'}), 404

@app.route('/api/show_library', methods=['GET'])
def show_library():
    return jsonify({
            'songs': [{
                'name': song.name,
                'uuid': song.uuid,
                'artists': song.artists,
                'album': song.album,
                'song_art_path': song.song_art_path,
                'is_liked': song.is_liked,
                'liked_time': song.liked_time,
                'file_path': song.file_path
            } for song in library.songs.values()],
            'albums': [{
                'name': album.name,
                'uuid': album.uuid,
                'year': album.year,
                'album_art_path': album.album_art_path,
                'is_liked': album.is_liked,
                'liked_time': album.liked_time
            } for album in library.albums.values()],
            'artists': [{
                'name': artist.name,
                'uuid': artist.uuid,
                'artist_art_path': artist.artist_art_path,
                'is_liked': artist.is_liked,
                'liked_time': artist.liked_time
            } for artist in library.artists.values()]
        }), 200

@app.route('/api/show_liked_songs', methods=['GET'])
def show_liked_songs():
    liked_songs = [{
        'name': song.name,
        'uuid': song.uuid,
        'artists': song.artists,
        'album': song.album,
        'song_art_path': song.song_art_path,
        'is_liked': song.is_liked,
        'liked_time': song.liked_time,
        'file_path': song.file_path
    } for song in library.songs.values() if song.is_liked]
    return jsonify(liked_songs), 200

@app.route('/api/show_liked_artists', methods=['GET'])
def show_liked_artists():
    liked_artists = [{
        'name': artist.name,
        'uuid': artist.uuid,
        'artist_art_path': artist.artist_art_path,
        'is_liked': artist.is_liked,
        'liked_time': artist.liked_time
    } for artist in library.artists.values() if artist.is_liked]
    return jsonify(liked_artists), 200

@app.route('/api/show_liked_albums', methods=['GET'])
def show_liked_albums():
    liked_albums = [{
        'name': album.name,
        'uuid': album.uuid,
        'year': album.year,
        'album_art_path': album.album_art_path,
        'is_liked': album.is_liked,
        'liked_time': album.liked_time
    } for album in library.albums.values() if album.is_liked]
    return jsonify(liked_albums), 200

@app.route('/api/show_song/<uuid>', methods=['GET'])
def show_song(uuid):
    song = library.goto_song(uuid)
    if song:
        return jsonify({
            'name': song.name,
            'uuid': song.uuid,
            'album': song.album,
            'artists': song.artists,
            'file_path': song.file_path,
            'track_number': song.track_number,
            'disc_number': song.disc_number,
            'is_liked': song.is_liked,
            'liked_time': song.liked_time,
            'song_art_path': song.song_art_path,
            'year': song.year,
            'event': song.event
        }), 200
    return jsonify({'message': 'Song not found'}), 404

@app.route('/api/show_album/<uuid>', methods=['GET'])
def show_album(uuid):
    album = library.goto_album(uuid)
    if album:
        return jsonify({
            'name': album.name,
            'uuid': album.uuid,
            'album_artists': [{'name': artist.name, 'uuid': artist.uuid} for artist in album.album_artists],
            'songs': [{'name': song.name, 'uuid': song.uuid, 'artists': song.artists, 'album': song.album, 'song_art_path': song.song_art_path, 'is_liked': song.is_liked, 'track_number': song.track_number, 'disc_number': song.disc_number} for song in album.songs],
            'is_liked': album.is_liked,
            'liked_time': album.liked_time,
            'album_art_path': album.album_art_path,
            'year': album.year,
            'event': album.event
        }), 200
    return jsonify({'message': 'Album not found'}), 404

@app.route('/api/show_artist/<uuid>', methods=['GET'])
def show_artist(uuid):
    artist = library.goto_artist(uuid)
    if artist:
        albums = [{'name': album.name, 'uuid': album.uuid, 'year': album.year, 'album_art_path': album.album_art_path, 'is_liked': album.is_liked} for album in library.albums.values() if artist in album.album_artists]
        songs = [{'name': song.name, 'uuid': song.uuid, 'artists': song.artists, 'album': song.album, 'song_art_path': song.song_art_path, 'is_liked': song.is_liked, 'track_number': song.track_number, 'disc_number': song.disc_number} for song in library.songs.values() if artist.uuid in [a['uuid'] for a in song.artists]]
        return jsonify({
            'name': artist.name,
            'uuid': artist.uuid,
            'albums': albums,
            'songs': songs,
            'is_liked': artist.is_liked,
            'liked_time': artist.liked_time,
            'artist_art_path': artist.artist_art_path
        }), 200
    return jsonify({'message': 'Artist not found'}), 404

@app.route('/api/search/<query>', methods=['GET'])
def search(query):
    results = library.search(query)
    return jsonify({
        'songs': [{
            'name': song.name,
            'uuid': song.uuid,
            'artists': song.artists,
            'album': song.album,
            'song_art_path': song.song_art_path,
            'is_liked': song.is_liked,
            'liked_time': song.liked_time,
            'file_path': song.file_path
        } for song in results['songs']],
        'albums': [{
            'name': album.name,
            'uuid': album.uuid,
            'year': album.year,
            'album_art_path': album.album_art_path,
            'is_liked': album.is_liked,
            'liked_time': album.liked_time
        } for album in results['albums']],
        'artists': [{
            'name': artist.name,
            'uuid': artist.uuid,
            'artist_art_path': artist.artist_art_path,
            'is_liked': artist.is_liked,
            'liked_time': artist.liked_time
        } for artist in results['artists']]
    }), 200

@app.route('/api/getfile', methods=['GET'])
def getfile():
    file_path = request.args.get('file_path')
    if not file_path or not os.path.exists(file_path):
        return jsonify({'message': 'File not found'}), 404

    return send_file(file_path, as_attachment=True)

@app.route('/api/getStream', methods=['GET'])
def get_stream():
    file_path = request.args.get('file_path')
    if not file_path or not os.path.exists(file_path):
        return jsonify({'message': 'File not found'}), 404

    if file_path.endswith('.mp3'):
        mimetype = 'audio/mpeg'
    elif file_path.endswith('.flac'):
        mimetype = 'audio/flac'
    else:
        return jsonify({'message': 'Unsupported audio format'}), 400

    return send_file(file_path, mimetype=mimetype)

@app.route('/api/show_relation', methods=['GET'])
def show_relation():
    artist_uuid = request.args.get('uuid')
    try:
        layer = int(request.args.get('layer'))
    except (TypeError, ValueError):
        return jsonify({'message': 'Invalid layer parameter'}), 400

    if not artist_uuid:
        return jsonify({'message': 'Artist UUID is required'}), 400

    relations_tree = library.show_relation(artist_uuid, layer)
    return jsonify(relations_tree), 200

@app.route('/api/merge_artist_by_uuid', methods=['GET'])
def merge_artist_by_uuid():
    uuid1 = request.args.get('uuid1')
    uuid2 = request.args.get('uuid2')

    if not uuid1 or not uuid2:
        return jsonify({'message': 'Both uuid1 and uuid2 are required'}), 400

    library.merge_artist_by_uuid(uuid1, uuid2)
    save_library()
    return jsonify({'message': f'Artist {uuid2} merged into {uuid1}'}), 200

@app.route('/api/merge_artist_by_name', methods=['GET'])
def merge_artist_by_name():
    name1 = request.args.get('name1')
    name2 = request.args.get('name2')

    if not name1 or not name2:
        return jsonify({'message': 'Both name1 and name2 are required'}), 400

    library.merge_artist_by_name(name1, name2)
    save_library()
    return jsonify({'message': f'Artist {name2} merged into {name1}'}), 200

@app.route('/api/auto_merge', methods=['GET'])
def auto_merge():
    library.auto_merge()
    save_library()
    return jsonify({'message': 'Auto merge completed'}), 200

@app.route('/api/lyrics', methods=['GET'])
def lyrics():
    title = request.args.get('title')
    artist = request.args.get('artist')
    album = request.args.get('album')
    duration = request.args.get('duration', type=float, default=0)

    local_lyrics = lrc.check_local_lyrics(title)
    if local_lyrics:
        response = jsonify([{
            "title": title,
            "artist": artist,
            "lyrics": local_lyrics
        }])
        response.headers['Content-Type'] = 'application/json'
        return response

    aligned_lyrics = lrc.get_aligned_lyrics(title, artist, album, duration)
    if aligned_lyrics:
        response = jsonify(aligned_lyrics)
        response.headers['Content-Type'] = 'application/json'
        return response
    else:
        response = Response('', status=404)
        response.headers['Content-Type'] = 'application/json'
        return response

@app.route('/', methods=['GET'])
def serve_static_index():
    print("index")
    return send_from_directory(static_folder, 'index.html')

@app.route('/<path:path>', methods=['GET'])
def serve_static(path):
    return send_from_directory(static_folder, path)

if __name__ == '__main__':
    load_library()
    app.run(debug=False, port=5010, host='0.0.0.0')
