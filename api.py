from flask import Flask, request, jsonify, send_file
from musiclib import MusicLibrary, Artist, Album, Song
import os
import atexit
import pickle
import signal

app = Flask(__name__)
library = MusicLibrary()
data_file = 'library_data.pkl'

# Function to load the library from a file
def load_library():
    global library
    try:
        if os.path.exists(data_file):
            with open(data_file, 'rb') as f:
                library = pickle.load(f)
                print("Library loaded from file")
        else:
            print("No library file found, starting with an empty library")
    except Exception as e:
        print(f"Failed to load library from file: {e}")

# Function to save the library to a file
def save_library():
    try:
        with open(data_file, 'wb') as f:
            pickle.dump(library, f)
            print("Library saved to file")
    except Exception as e:
        print(f"Failed to save library to file: {e}")

# Load the library at startup
load_library()

# Register the save_library function to be called on exit
atexit.register(save_library)

# Handle SIGINT (Ctrl+C) to ensure the library is saved
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
    
    album = library.goto_album(album_uuid)
    artists = [library.goto_artist(uuid) for uuid in artist_uuids]
    
    song = Song(name, album, artists, file_path, track_number, disc_number)
    library.add_song(song)
    save_library()  # Save after adding
    return jsonify({'message': 'Song added'}), 201

@app.route('/add_album', methods=['GET'])
def add_album():
    name = request.args.get('name')
    album = Album(name)
    library.add_album(album)
    save_library()  # Save after adding
    return jsonify({'message': 'Album added', 'uuid': album.uuid}), 201

@app.route('/add_artist', methods=['GET'])
def add_artist():
    name = request.args.get('name')
    artist = Artist(name)
    library.add_artist(artist)
    save_library()  # Save after adding
    return jsonify({'message': 'Artist added', 'uuid': artist.uuid}), 201

@app.route('/like_song/<uuid>', methods=['GET'])
def like_song(uuid):
    library.like_song(uuid)
    save_library()  # Save after liking
    return jsonify({'message': 'Song liked'}), 200

@app.route('/unlike_song/<uuid>', methods=['GET'])
def unlike_song(uuid):
    library.unlike_song(uuid)
    save_library()  # Save after unliking
    return jsonify({'message': 'Song unliked'}), 200

@app.route('/like_album/<uuid>', methods=['GET'])
def like_album(uuid):
    library.like_album(uuid)
    save_library()  # Save after liking
    return jsonify({'message': 'Album liked'}), 200

@app.route('/unlike_album/<uuid>', methods=['GET'])
def unlike_album(uuid):
    library.unlike_album(uuid)
    save_library()  # Save after unliking
    return jsonify({'message': 'Album unliked'}), 200

@app.route('/like_artist/<uuid>', methods=['GET'])
def like_artist(uuid):
    library.like_artist(uuid)
    save_library()  # Save after liking
    return jsonify({'message': 'Artist liked'}), 200

@app.route('/unlike_artist/<uuid>', methods=['GET'])
def unlike_artist(uuid):
    library.unlike_artist(uuid)
    save_library()  # Save after unliking
    return jsonify({'message': 'Artist unliked'}), 200

@app.route('/scan', methods=['GET'])
def scan():
    directory = request.args.get('directory')
    if not directory:
        return jsonify({'message': 'Directory is required'}), 400

    library.scan(directory)
    save_library()  # Save after scanning
    return jsonify({'message': 'Scan completed'}), 200

# The rest of the routes remain the same

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
        'artists': {artist.uuid: artist.name for artist in library.artists.values()},
        'albums': {album.uuid: album.name for album in library.albums.values()},
        'songs': {song.uuid: song.name for song in library.songs.values()}
    }), 200

@app.route('/show_liked_songs', methods=['GET'])
def show_liked_songs():
    liked_songs = [{'name': song.name, 'uuid': song.uuid} for song in library.songs.values() if song.is_liked]
    return jsonify(liked_songs), 200

@app.route('/show_liked_artists', methods=['GET'])
def show_liked_artists():
    liked_artists = [{'name': artist.name, 'uuid': artist.uuid} for artist in library.artists.values() if artist.is_liked]
    return jsonify(liked_artists), 200

@app.route('/show_liked_albums', methods=['GET'])
def show_liked_albums():
    liked_albums = [{'name': album.name, 'uuid': album.uuid} for album in library.albums.values() if album.is_liked]
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
            'song_art_path': song.song_art_path
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
            'songs': [{'name': song.name, 'uuid': song.uuid, 'track_number': song.track_number, 'disc_number': song.disc_number} for song in album.songs],
            'is_liked': album.is_liked,
            'album_art_path': album.album_art_path
        }), 200
    return jsonify({'message': 'Album not found'}), 404

@app.route('/show_artist/<uuid>', methods=['GET'])
def show_artist(uuid):
    artist = library.goto_artist(uuid)
    if artist:
        albums = [{'name': album.name, 'uuid': album.uuid} for album in library.albums.values() if artist in album.album_artists]
        songs = [{'name': song.name, 'uuid': song.uuid, 'track_number': song.track_number, 'disc_number': song.disc_number} for song in library.songs.values() if artist.uuid in [a['uuid'] for a in song.artists]]
        return jsonify({
            'name': artist.name,
            'uuid': artist.uuid,
            'albums': albums,
            'songs': songs,
            'is_liked': artist.is_liked,
            'artist_art_path': artist.artist_art_path
        }), 200
    return jsonify({'message': 'Artist not found'}), 404

@app.route('/search/<query>', methods=['GET'])
def search(query):
    results = library.search(query)
    return jsonify({
        'songs': [{'name': song.name, 'uuid': song.uuid} for song in results['songs']],
        'albums': [{'name': album.name, 'uuid': album.uuid} for album in results['albums']],
        'artists': [{'name': artist.name, 'uuid': artist.uuid} for artist in results['artists']]
    }), 200

@app.route('/getfile', methods=['GET'])
def getfile():
    file_path = request.args.get('file_path')
    if not file_path or not os.path.exists(file_path):
        return jsonify({'message': 'File not found'}), 404

    return send_file(file_path, as_attachment=True)

if __name__ == '__main__':
    app.run(debug=True)