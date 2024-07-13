from flask import Flask, request, jsonify, send_file
from musiclib import MusicLibrary, Artist, Album, Song
import os
import atexit
import signal
from flask_cors import CORS
from flask_compress import Compress
import json
import opencc


app = Flask(__name__)
CORS(app)
Compress(app)

library = MusicLibrary()
data_file = 'library_data.json'

def save_library():
    """Saves the current state of the music library to a JSON file."""
    data = {
        'artists': {uuid: artist.__dict__ for uuid, artist in library.artists.items()},
        'albums': {uuid: {
            **album.__dict__,
            'album_artists': [artist.uuid for artist in album.album_artists],
            'songs': [song.uuid for song in album.songs]
        } for uuid, album in library.albums.items()},
        'songs': {uuid: {
            **song.__dict__,
            'album': song.album['uuid'],
            'artists': [artist['uuid'] for artist in song.artists]
        } for uuid, song in library.songs.items()},
        'graph': {k: {kk: {'strength': vv['strength'], 'details': list(vv['details'])} for kk, vv in v.items()} for k, v in library.graph.items()}  # Convert sets to lists for JSON serialization
    }

    try:
        with open(data_file, 'w', encoding='utf-8') as file:
            json.dump(data, file, ensure_ascii=False, indent=4)
        print("Library saved to file")
    except Exception as e:
        print(f"Failed to save library to file: {e}")

def load_library():
    """Loads the music library state from a JSON file."""
    global library
    try:
        if os.path.exists(data_file):
            with open(data_file, 'r', encoding='utf-8') as file:
                data = json.load(file)

            library = MusicLibrary()
            
            # Restore artists
            for uuid, artist_data in data['artists'].items():
                artist = Artist(artist_data['name'])
                artist.uuid = artist_data['uuid']
                artist.is_liked = artist_data['is_liked']
                artist.liked_time = artist_data['liked_time']
                artist.artist_art_path = artist_data['artist_art_path']
                library.artists[uuid] = artist

            # Restore albums
            for uuid, album_data in data['albums'].items():
                album = Album(album_data['name'])
                album.uuid = album_data['uuid']
                album.is_liked = album_data['is_liked']
                album.liked_time = album_data['liked_time']
                album.album_art_path = album_data['album_art_path']
                album.year = album_data['year']
                album.album_artists = {library.artists[artist_uuid] for artist_uuid in album_data['album_artists']}
                album.songs = []
                library.albums[uuid] = album

            # Restore songs
            for uuid, song_data in data['songs'].items():
                album = library.albums[song_data['album']]
                artists = [library.artists[artist_uuid] for artist_uuid in song_data['artists']]
                song = Song(
                    song_data['name'], album, artists, song_data['file_path'],
                    song_data['track_number'], song_data['disc_number'], song_data['year']
                )
                song.uuid = song_data['uuid']
                song.is_liked = song_data['is_liked']
                song.liked_time = song_data['liked_time']
                song.song_art_path = song_data['song_art_path']
                library.songs[uuid] = song
                album.songs.append(song)

            # Restore graph
            library.graph = {k: {kk: {'strength': vv['strength'], 'details': set(vv['details'])} for kk, vv in v.items()} for k, v in data.get('graph', {}).items()}

            # Reinitialize OpenCC object
            library.cc = opencc.OpenCC('t2s')

            print("Library loaded from file")
        else:
            print("No library file found, starting with an empty library")
    except Exception as e:
        print(f"Failed to load library from file: {e}")
        
load_library()

atexit.register(save_library)

def handle_sigint(signal, frame):
    print("SIGINT received. Saving library and exiting...")
    save_library()
    os._exit(0)

signal.signal(signal.SIGINT, handle_sigint)

@app.route('/add_song', methods=['GET'])
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

@app.route('/add_album', methods=['GET'])
def add_album():
    name = request.args.get('name')
    year = request.args.get('year')
    album = Album(name)
    album.year = year
    library.add_album(album)
    save_library()
    return jsonify({'message': 'Album added', 'uuid': album.uuid, 'year': album.year}), 201

@app.route('/add_artist', methods=['GET'])
def add_artist():
    name = request.args.get('name')
    artist = Artist(name)
    library.add_artist(artist)
    save_library()
    return jsonify({'message': 'Artist added', 'uuid': artist.uuid}), 201

@app.route('/like_song/<uuid>', methods=['GET'])
def like_song(uuid):
    library.like_song(uuid)
    save_library()
    return jsonify({'message': 'Song liked'}), 200

@app.route('/unlike_song/<uuid>', methods=['GET'])
def unlike_song(uuid):
    library.unlike_song(uuid)
    save_library()
    return jsonify({'message': 'Song unliked'}), 200

@app.route('/like_album/<uuid>', methods=['GET'])
def like_album(uuid):
    library.like_album(uuid)
    save_library()
    return jsonify({'message': 'Album liked'}), 200

@app.route('/unlike_album/<uuid>', methods=['GET'])
def unlike_album(uuid):
    library.unlike_album(uuid)
    save_library()
    return jsonify({'message': 'Album unliked'}), 200

@app.route('/like_artist/<uuid>', methods=['GET'])
def like_artist(uuid):
    library.like_artist(uuid)
    save_library()
    return jsonify({'message': 'Artist liked'}), 200

@app.route('/unlike_artist/<uuid>', methods=['GET'])
def unlike_artist(uuid):
    library.unlike_artist(uuid)
    save_library()
    return jsonify({'message': 'Artist unliked'}), 200

@app.route('/scan', methods=['GET'])
def scan():
    directory = request.args.get('directory')
    if not directory:
        return jsonify({'message': 'Directory is required'}), 400

    library.scan(directory)
    save_library()
    return jsonify({'message': 'Scan completed'}), 200

@app.route('/search_song/<name>', methods=['GET'])
def search_song(name):
    song = library.search_song(name)
    if song:
        return jsonify({'name': song.name, 'uuid': song.uuid}), 200
    return jsonify({'message': 'Song not found'}), 404

@app.route('/search_album/<name>', methods=['GET'])
def search_album(name):
    album = library.search_album(name)
    if album:
        return jsonify({'name': album.name, 'uuid': album.uuid}), 200
    return jsonify({'message': 'Album not found'}), 404

@app.route('/search_artist/<name>', methods=['GET'])
def search_artist(name):
    artist = library.search_artist(name)
    if artist:
        return jsonify({'name': artist.name, 'uuid': artist.uuid}), 200
    return jsonify({'message': 'Artist not found'}), 404

@app.route('/show_library', methods=['GET'])
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

@app.route('/show_liked_songs', methods=['GET'])
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

@app.route('/show_liked_artists', methods=['GET'])
def show_liked_artists():
    liked_artists = [{
        'name': artist.name, 
        'uuid': artist.uuid, 
        'artist_art_path': artist.artist_art_path, 
        'is_liked': artist.is_liked,
        'liked_time': artist.liked_time
    } for artist in library.artists.values() if artist.is_liked]
    return jsonify(liked_artists), 200

@app.route('/show_liked_albums', methods=['GET'])
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

@app.route('/show_song/<uuid>', methods=['GET'])
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
            'year': song.year
        }), 200
    return jsonify({'message': 'Song not found'}), 404

@app.route('/show_album/<uuid>', methods=['GET'])
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
            'year': album.year
        }), 200
    return jsonify({'message': 'Album not found'}), 404

@app.route('/show_artist/<uuid>', methods=['GET'])
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

@app.route('/search/<query>', methods=['GET'])
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

@app.route('/getfile', methods=['GET'])
def getfile():
    file_path = request.args.get('file_path')
    if not file_path or not os.path.exists(file_path):
        return jsonify({'message': 'File not found'}), 404

    return send_file(file_path, as_attachment=True)

@app.route('/getStream', methods=['GET'])
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

@app.route('/show_relation', methods=['GET'])
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

@app.route('/merge_artist_by_uuid', methods=['GET'])
def merge_artist_by_uuid():
    uuid1 = request.args.get('uuid1')
    uuid2 = request.args.get('uuid2')
    
    if not uuid1 or not uuid2:
        return jsonify({'message': 'Both uuid1 and uuid2 are required'}), 400
    
    library.merge_artist_by_uuid(uuid1, uuid2)
    save_library()
    return jsonify({'message': f'Artist {uuid2} merged into {uuid1}'}), 200

@app.route('/merge_artist_by_name', methods=['GET'])
def merge_artist_by_name():
    name1 = request.args.get('name1')
    name2 = request.args.get('name2')
    
    if not name1 or not name2:
        return jsonify({'message': 'Both name1 and name2 are required'}), 400

    library.merge_artist_by_name(name1, name2)
    save_library()
    return jsonify({'message': f'Artist {name2} merged into {name1}'}), 200

@app.route('/auto_merge', methods=['GET'])
def auto_merge():
    library.auto_merge()
    save_library()
    return jsonify({'message': 'Auto merge completed'}), 200

if __name__ == '__main__':
    app.run(debug=True, port=5010, host='0.0.0.0')