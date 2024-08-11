from flask import Flask, Response, request, jsonify, send_file, send_from_directory
from library_save_load import load_library, save_library
from lyrics import LyricsSource
from music_library import Event, Artist, Album, Song
import os
from flask_cors import CORS
from flask_compress import Compress

app = Flask(__name__)
CORS(app)
Compress(app)

current_dir = os.path.dirname(os.path.abspath(__file__))
static_folder = os.path.join(current_dir, "webui")


@app.route("/api/add_artist", methods=["GET"])
def add_artist():
    name = request.args.get("name")
    artist = Artist(name)
    library.add_artist(artist)
    save_library(library)
    return jsonify({"message": "Artist added", "uuid": artist.uuid}), 201


@app.route("/api/like_event/<uuid>", methods=["GET"])
def like_event(uuid):
    event = library.events.get(uuid)
    if event:
        event.like()
        save_library(library)
        return jsonify({"message": "Event liked"}), 200
    return jsonify({"message": "Event not found"}), 404


@app.route("/api/unlike_event/<uuid>", methods=["GET"])
def unlike_event(uuid):
    event = library.events.get(uuid)
    if event:
        event.unlike()
        save_library(library)
        return jsonify({"message": "Event unliked"}), 200
    return jsonify({"message": "Event not found"}), 404


@app.route("/api/like_song/<uuid>", methods=["GET"])
def like_song(uuid):
    library.like_song(uuid)
    save_library(library)
    return jsonify({"message": "Song liked"}), 200


@app.route("/api/unlike_song/<uuid>", methods=["GET"])
def unlike_song(uuid):
    library.unlike_song(uuid)
    save_library(library)
    return jsonify({"message": "Song unliked"}), 200


@app.route("/api/like_album/<uuid>", methods=["GET"])
def like_album(uuid):
    library.like_album(uuid)
    save_library(library)
    return jsonify({"message": "Album liked"}), 200


@app.route("/api/unlike_album/<uuid>", methods=["GET"])
def unlike_album(uuid):
    library.unlike_album(uuid)
    save_library(library)
    return jsonify({"message": "Album unliked"}), 200


@app.route("/api/like_artist/<uuid>", methods=["GET"])
def like_artist(uuid):
    library.like_artist(uuid)
    save_library(library)
    return jsonify({"message": "Artist liked"}), 200


@app.route("/api/unlike_artist/<uuid>", methods=["GET"])
def unlike_artist(uuid):
    library.unlike_artist(uuid)
    save_library(library)
    return jsonify({"message": "Artist unliked"}), 200


@app.route("/api/scan", methods=["GET"])
def scan():
    directory = request.args.get("directory")
    if not directory:
        return jsonify({"message": "Directory is required"}), 400

    library.scan(directory)
    save_library(library)
    return jsonify({"message": "Scan completed"}), 200


@app.route("/api/show_event/<uuid>", methods=["GET"])
def show_event(uuid):
    event = library.events.get(uuid)
    if event:
        return jsonify(
            {
                "name": event.name,
                "uuid": event.uuid,
                "year": event.year,
                "date": event.date,
                "albums": [
                    {
                        "name": album.name,
                        "uuid": album.uuid,
                        "album_art_path": album.album_art_path,
                        "album_artists": [
                            {"name": artist.name, "uuid": artist.uuid}
                            for artist in album.album_artists
                        ],
                    }
                    for album in event.albums
                ],
                "is_liked": event.is_liked,
                "liked_time": event.liked_time,
            }
        ), 200
    return jsonify({"message": "Event not found"}), 404


@app.route("/api/show_library", methods=["GET"])
def show_library():
    return jsonify(
        {
            "songs": [
                {
                    "name": song.name,
                    "uuid": song.uuid,
                    "artists": song.artists,
                    "album": song.album,
                    "song_art_path": song.song_art_path,
                    "is_liked": song.is_liked,
                    "liked_time": song.liked_time,
                    "file_path": song.file_path,
                }
                for song in library.songs.values()
            ],
            "albums": [
                {
                    "name": album.name,
                    "uuid": album.uuid,
                    "year": album.year,
                    "date": album.date,
                    "album_art_path": album.album_art_path,
                    "is_liked": album.is_liked,
                    "liked_time": album.liked_time,
                }
                for album in library.albums.values()
            ],
            "artists": [
                {
                    "name": artist.name,
                    "uuid": artist.uuid,
                    "artist_art_path": artist.artist_art_path,
                    "is_liked": artist.is_liked,
                    "liked_time": artist.liked_time,
                }
                for artist in library.artists.values()
            ],
        }
    ), 200


@app.route("/api/show_liked_events", methods=["GET"])
def show_liked_events():
    liked_events = [
        {
            "name": event.name,
            "uuid": event.uuid,
            "year": event.year,
            "date": event.date,
            "albums": [
                {"name": album.name, "uuid": album.uuid} for album in event.albums
            ],
            "is_liked": event.is_liked,
            "liked_time": event.liked_time,
        }
        for event in library.events.values()
        if event.is_liked
    ]
    return jsonify(liked_events), 200


@app.route("/api/show_liked_songs", methods=["GET"])
def show_liked_songs():
    liked_songs = [
        {
            "name": song.name,
            "uuid": song.uuid,
            "artists": song.artists,
            "album": song.album,
            "song_art_path": song.song_art_path,
            "is_liked": song.is_liked,
            "liked_time": song.liked_time,
            "file_path": song.file_path,
        }
        for song in library.songs.values()
        if song.is_liked
    ]
    return jsonify(liked_songs), 200


@app.route("/api/show_liked_artists", methods=["GET"])
def show_liked_artists():
    liked_artists = [
        {
            "name": artist.name,
            "uuid": artist.uuid,
            "artist_art_path": artist.artist_art_path,
            "is_liked": artist.is_liked,
            "liked_time": artist.liked_time,
        }
        for artist in library.artists.values()
        if artist.is_liked
    ]
    return jsonify(liked_artists), 200


@app.route("/api/show_liked_albums", methods=["GET"])
def show_liked_albums():
    liked_albums = [
        {
            "name": album.name,
            "uuid": album.uuid,
            "year": album.year,
            "date": album.date,
            "album_art_path": album.album_art_path,
            "is_liked": album.is_liked,
            "liked_time": album.liked_time,
        }
        for album in library.albums.values()
        if album.is_liked
    ]
    return jsonify(liked_albums), 200


@app.route("/api/show_song/<uuid>", methods=["GET"])
def show_song(uuid):
    song = library.goto_song(uuid)
    if song:
        return jsonify(
            {
                "name": song.name,
                "uuid": song.uuid,
                "album": song.album,
                "artists": song.artists,
                "file_path": song.file_path,
                "track_number": song.track_number,
                "disc_number": song.disc_number,
                "is_liked": song.is_liked,
                "liked_time": song.liked_time,
                "song_art_path": song.song_art_path,
                "year": song.year,
                "date": song.date,
                "event": song.event,
            }
        ), 200
    return jsonify({"message": "Song not found"}), 404


@app.route("/api/show_album/<uuid>", methods=["GET"])
def show_album(uuid):
    album = library.goto_album(uuid)
    if album:
        return jsonify(
            {
                "name": album.name,
                "uuid": album.uuid,
                "album_artists": [
                    {"name": artist.name, "uuid": artist.uuid}
                    for artist in album.album_artists
                ],
                "songs": [
                    {
                        "name": song.name,
                        "uuid": song.uuid,
                        "artists": song.artists,
                        "album": song.album,
                        "song_art_path": song.song_art_path,
                        "is_liked": song.is_liked,
                        "track_number": song.track_number,
                        "disc_number": song.disc_number,
                    }
                    for song in album.songs
                ],
                "is_liked": album.is_liked,
                "liked_time": album.liked_time,
                "album_art_path": album.album_art_path,
                "year": album.year,
                "date": album.date,
                "event": album.event,
            }
        ), 200
    return jsonify({"message": "Album not found"}), 404


@app.route("/api/show_artist/<uuid>", methods=["GET"])
def show_artist(uuid):
    artist = library.goto_artist(uuid)
    if artist:
        albums = sorted(
            [
                {
                    "name": album.name,
                    "uuid": album.uuid,
                    "year": album.year,
                    "date": album.date,
                    "album_art_path": album.album_art_path,
                    "is_liked": album.is_liked,
                }
                for album in library.albums.values()
                if artist in album.album_artists
            ],
            key=lambda x: (
                x["year"] is None,
                -x["year"] if x["year"] is not None else 0,
            ),
        )
        songs = [
            {
                "name": song.name,
                "uuid": song.uuid,
                "artists": song.artists,
                "album": song.album,
                "song_art_path": song.song_art_path,
                "is_liked": song.is_liked,
                "track_number": song.track_number,
                "disc_number": song.disc_number,
            }
            for song in library.songs.values()
            if artist.uuid in [a["uuid"] for a in song.artists]
        ]
        return jsonify(
            {
                "name": artist.name,
                "uuid": artist.uuid,
                "albums": albums,
                "songs": songs,
                "is_liked": artist.is_liked,
                "liked_time": artist.liked_time,
                "artist_art_path": artist.artist_art_path,
            }
        ), 200
    return jsonify({"message": "Artist not found"}), 404


@app.route("/api/search/<query>", methods=["GET"])
def search(query):
    results = library.searcher.search(query)
    return jsonify(
        {
            "songs": [
                {
                    "name": song.name,
                    "uuid": song.uuid,
                    "artists": song.artists,
                    "album": song.album,
                    "song_art_path": song.song_art_path,
                    "is_liked": song.is_liked,
                    "liked_time": song.liked_time,
                    "file_path": song.file_path,
                }
                for song in results["songs"]
            ],
            "albums": [
                {
                    "name": album.name,
                    "uuid": album.uuid,
                    "year": album.year,
                    "date": album.date,
                    "album_art_path": album.album_art_path,
                    "is_liked": album.is_liked,
                    "liked_time": album.liked_time,
                }
                for album in results["albums"]
            ],
            "artists": [
                {
                    "name": artist.name,
                    "uuid": artist.uuid,
                    "artist_art_path": artist.artist_art_path,
                    "is_liked": artist.is_liked,
                    "liked_time": artist.liked_time,
                }
                for artist in results["artists"]
            ],
        }
    ), 200


@app.route("/api/getfile", methods=["GET"])
def getfile():
    file_path = request.args.get("file_path")
    if not file_path or not os.path.exists(file_path):
        return jsonify({"message": "File not found"}), 404

    # Extract file name without extension and check if it's 'folder' or 'cover'
    file_name_without_ext = os.path.splitext(os.path.basename(file_path))[0].lower()
    if file_name_without_ext not in ["folder", "cover"]:
        return jsonify({"message": "Invalid file name"}), 400

    # Check if the file is an image
    if not file_path.lower().endswith((".png", ".jpg", ".jpeg", ".tif", ".tiff")):
        return jsonify({"message": "File is not an image"}), 400

    response = send_file(file_path, as_attachment=True)

    return response


@app.route("/api/getStream", methods=["GET"])
def get_stream():
    file_path = request.args.get("file_path")
    if not file_path or not os.path.exists(file_path):
        return jsonify({"message": "File not found"}), 404

    if file_path.endswith(".mp3"):
        mimetype = "audio/mpeg"
    elif file_path.endswith(".flac"):
        mimetype = "audio/flac"
    else:
        return jsonify({"message": "Unsupported audio format"}), 400

    return send_file(file_path, mimetype=mimetype)


@app.route("/api/show_relation", methods=["GET"])
def show_relation():
    artist_uuid = request.args.get("uuid")
    try:
        layer = int(request.args.get("layer"))
    except (TypeError, ValueError):
        return jsonify({"message": "Invalid layer parameter"}), 400

    if not artist_uuid:
        return jsonify({"message": "Artist UUID is required"}), 400

    relations_tree = library.show_relation(artist_uuid, layer)
    return jsonify(relations_tree), 200


@app.route("/api/merge_artist_by_uuid", methods=["GET"])
def merge_artist_by_uuid():
    uuid1 = request.args.get("uuid1")
    uuid2 = request.args.get("uuid2")

    if not uuid1 or not uuid2:
        return jsonify({"message": "Both uuid1 and uuid2 are required"}), 400

    library.merge_artist_by_uuid(uuid1, uuid2)
    save_library(library)
    return jsonify({"message": f"Artist {uuid2} merged into {uuid1}"}), 200


@app.route("/api/merge_artist_by_name", methods=["GET"])
def merge_artist_by_name():
    name1 = request.args.get("name1")
    name2 = request.args.get("name2")

    if not name1 or not name2:
        return jsonify({"message": "Both name1 and name2 are required"}), 400

    library.merge_artist_by_name(name1, name2)
    save_library(library)
    return jsonify({"message": f"Artist {name2} merged into {name1}"}), 200


@app.route("/api/auto_merge", methods=["GET"])
def auto_merge():
    library.auto_merge()
    save_library(library)
    return jsonify({"message": "Auto merge completed"}), 200


@app.route("/api/lyrics", methods=["GET"])
def lyrics():
    title = request.args.get("title")
    artist = request.args.get("artist")
    album = request.args.get("album")
    duration = request.args.get("duration", type=float, default=0)

    aligned_lyrics = LyricsSource.get_lyrics(title, artist, album, duration)
    if aligned_lyrics:
        response = jsonify(aligned_lyrics)
        response.headers["Content-Type"] = "application/json"
        return response
    else:
        response = Response("", status=404)
        response.headers["Content-Type"] = "application/json"
        return response


@app.route("/", methods=["GET"])
def serve_static_index():
    print("index")
    return send_from_directory(static_folder, "index.html")


@app.route("/<path:path>", methods=["GET"])
def serve_static(path):
    return send_from_directory(static_folder, path)


if __name__ == "__main__":
    global library
    library = load_library()
    app.run(debug=False, port=5010, host="0.0.0.0")
