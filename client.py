import requests
from cmd import Cmd
import os
import json
import subprocess
from pygments import highlight, lexers, formatters

class MusicLibraryClient:
    def __init__(self, base_url="http://127.0.0.1:5010"):
        self.base_url = base_url
        self.headers = {"Content-Type": "application/json"}

    def add_song(self, name, album_uuid, artist_uuids, file_path, track_number=1, disc_number=1, year=None):
        params = {
            "name": name,
            "album_uuid": album_uuid,
            "artist_uuids": artist_uuids,
            "file_path": file_path,
            "track_number": track_number,
            "disc_number": disc_number,
            "year": year
        }
        response = requests.get(f"{self.base_url}/add_song", params=params, headers=self.headers)
        return response.json()

    def add_album(self, name, year=None):
        params = {"name": name, "year": year}
        response = requests.get(f"{self.base_url}/add_album", params=params, headers=self.headers)
        return response.json()

    def add_artist(self, name):
        params = {"name": name}
        response = requests.get(f"{self.base_url}/add_artist", params=params, headers=self.headers)
        return response.json()

    def search_song(self, name):
        response = requests.get(f"{self.base_url}/search_song/{name}", headers=self.headers)
        return response.json()

    def search_album(self, name):
        response = requests.get(f"{self.base_url}/search_album/{name}", headers=self.headers)
        return response.json()

    def search_artist(self, name):
        response = requests.get(f"{self.base_url}/search_artist/{name}", headers=self.headers)
        return response.json()

    def like_song(self, uuid):
        response = requests.get(f"{self.base_url}/like_song/{uuid}", headers=self.headers)
        return response.json()

    def unlike_song(self, uuid):
        response = requests.get(f"{self.base_url}/unlike_song/{uuid}", headers=self.headers)
        return response.json()

    def like_album(self, uuid):
        response = requests.get(f"{self.base_url}/like_album/{uuid}", headers=self.headers)
        return response.json()

    def unlike_album(self, uuid):
        response = requests.get(f"{self.base_url}/unlike_album/{uuid}", headers=self.headers)
        return response.json()

    def like_artist(self, uuid):
        response = requests.get(f"{self.base_url}/like_artist/{uuid}", headers=self.headers)
        return response.json()

    def unlike_artist(self, uuid):
        response = requests.get(f"{self.base_url}/unlike_artist/{uuid}", headers=self.headers)
        return response.json()

    def show_library(self):
        response = requests.get(f"{self.base_url}/show_library", headers=self.headers)
        return response.json()

    def show_liked_songs(self):
        response = requests.get(f"{self.base_url}/show_liked_songs", headers=self.headers)
        return response.json()

    def show_liked_artists(self):
        response = requests.get(f"{self.base_url}/show_liked_artists", headers=self.headers)
        return response.json()

    def show_liked_albums(self):
        response = requests.get(f"{self.base_url}/show_liked_albums", headers=self.headers)
        return response.json()

    def scan(self, directory):
        params = {"directory": directory}
        response = requests.get(f"{self.base_url}/scan", params=params, headers=self.headers)
        return response.json()

    def show_song(self, uuid):
        response = requests.get(f"{self.base_url}/show_song/{uuid}", headers=self.headers)
        return response.json()

    def show_album(self, uuid):
        response = requests.get(f"{self.base_url}/show_album/{uuid}", headers=self.headers)
        return response.json()

    def show_artist(self, uuid):
        response = requests.get(f"{self.base_url}/show_artist/{uuid}", headers=self.headers)
        return response.json()

    def search(self, query):
        response = requests.get(f"{self.base_url}/search/{query}", headers=self.headers)
        return response.json()

    def get_file(self, file_path):
        params = {"file_path": file_path}
        response = requests.get(f"{self.base_url}/getfile", params=params, headers=self.headers)
        return response.content

class MusicLibraryConsole(Cmd):
    prompt = "MusicLibrary> "
    client = MusicLibraryClient()

    def pretty_print_json(self, data):
        formatted_json = json.dumps(data, indent=4, ensure_ascii=False)
        colorful_json = highlight(formatted_json, lexers.JsonLexer(), formatters.TerminalFormatter())
        print(colorful_json)

    def print_song_details(self, song_info):
        print(f"Song: {song_info.get('name')}")
        print(f"UUID: {song_info.get('uuid')}")
        album = song_info.get('album', {})
        print(f"Album: {album.get('name')} (UUID: {album.get('uuid')})")
        artists = song_info.get('artists', [])
        for artist in artists:
            print(f"Artist: {artist.get('name')} (UUID: {artist.get('uuid')})")
        print(f"File Path: {song_info.get('file_path')}")
        print(f"Track Number: {song_info.get('track_number')}")
        print(f"Disc Number: {song_info.get('disc_number')}")
        print(f"Song Art Path: {song_info.get('song_art_path')}")
        print(f"Is Liked: {song_info.get('is_liked')}")
        print(f"Year: {song_info.get('year')}")

    def print_album_details(self, album_info):
        print(f"Album: {album_info.get('name')}")
        print(f"UUID: {album_info.get('uuid')}")
        album_artists = album_info.get('album_artists', [])
        for artist in album_artists:
            print(f"Artist: {artist.get('name')} (UUID: {artist.get('uuid')})")
        songs = album_info.get('songs', [])
        for song in songs:
            print(f"Song: {song.get('name')} (UUID: {song.get('uuid')})")
            print(f"  Track Number: {song.get('track_number')}")
            print(f"  Disc Number: {song.get('disc_number')}")
        print(f"Album Art Path: {album_info.get('album_art_path')}")
        print(f"Is Liked: {album_info.get('is_liked')}")
        print(f"Year: {album_info.get('year')}")

    def print_artist_details(self, artist_info):
        print(f"Artist: {artist_info.get('name')}")
        print(f"UUID: {artist_info.get('uuid')}")
        albums = artist_info.get('albums', [])
        for album in albums:
            print(f"Album: {album.get('name')} (UUID: {album.get('uuid')})")
        songs = artist_info.get('songs', [])
        for song in songs:
            print(f"Song: {song.get('name')} (UUID: {song.get('uuid')})")
            print(f"  Track Number: {song.get('track_number')}")
            print(f"  Disc Number: {song.get('disc_number')}")
        print(f"Artist Art Path: {artist_info.get('artist_art_path')}")
        print(f"Is Liked: {artist_info.get('is_liked')}")

    def print_library_details(self, library_info):
        print("Artists:")
        for uuid, name in library_info.get('artists', {}).items():
            print(f"  {name} (UUID: {uuid})")
        print("\nAlbums:")
        for uuid, name in library_info.get('albums', {}).items():
            print(f"  {name} (UUID: {uuid})")
        print("\nSongs:")
        for uuid, name in library_info.get('songs', {}).items():
            print(f"  {name} (UUID: {uuid})")

    def print_search_results(self, search_results):
        print("Songs:")
        for song in search_results.get('songs', []):
            print(f"  {song.get('name')} (UUID: {song.get('uuid')})")
        print("\nAlbums:")
        for album in search_results.get('albums', []):
            print(f"  {album.get('name')} (UUID: {album.get('uuid')})")
        print("\nArtists:")
        for artist in search_results.get('artists', []):
            print(f"  {artist.get('name')} (UUID: {artist.get('uuid')})")

    def print_liked_songs(self, liked_songs):
        print("Liked Songs:")
        for song in liked_songs:
            print(f"  {song.get('name')} (UUID: {song.get('uuid')})")

    def print_liked_artists(self, liked_artists):
        print("Liked Artists:")
        for artist in liked_artists:
            print(f"  {artist.get('name')} (UUID: {artist.get('uuid')})")

    def print_liked_albums(self, liked_albums):
        print("Liked Albums:")
        for album in liked_albums:
            print(f"  {album.get('name')} (UUID: {album.get('uuid')})")

    def do_search_album(self, args):
        """Search for an album by name: search_album <name>"""
        if not args:
            print("Usage: search_album <name>")
            return
        result = self.client.search_album(args)
        self.print_album_details(result)

    def do_search_artist(self, args):
        """Search for an artist by name: search_artist <name>"""
        if not args:
            print("Usage: search_artist <name>")
            return
        result = self.client.search_artist(args)
        self.print_artist_details(result)

    def do_search_song(self, args):
        """Search for a song by name: search_song <name>"""
        if not args:
            print("Usage: search_song <name>")
            return
        result = self.client.search_song(args)
        self.print_song_details(result)

    def do_show_library(self):
        """Show the entire music library: show_library"""
        result = self.client.show_library()
        self.print_library_details(result)

    def do_show_album(self, args):
        """Show album details by UUID: show_album <uuid>"""
        if not args:
            print("Usage: show_album <uuid>")
            return
        result = self.client.show_album(args)
        self.print_album_details(result)

    def do_show_artist(self, args):
        """Show artist details by UUID: show_artist <uuid>"""
        if not args:
            print("Usage: show_artist <uuid>")
            return
        result = self.client.show_artist(args)
        self.print_artist_details(result)

    def do_show_song(self, args):
        """Show song details by UUID: show_song <uuid>"""
        if not args:
            print("Usage: show_song <uuid>")
            return
        result = self.client.show_song(args)
        self.print_song_details(result)

    def do_show_liked_songs(self):
        """Show liked songs: show_liked_songs"""
        result = self.client.show_liked_songs()
        self.print_liked_songs(result)

    def do_show_liked_artists(self):
        """Show liked artists: show_liked_artists"""
        result = self.client.show_liked_artists()
        self.print_liked_artists(result)

    def do_show_liked_albums(self):
        """Show liked albums: show_liked_albums"""
        result = self.client.show_liked_albums()
        self.print_liked_albums(result)

    def do_play_song(self, args):
        """Play a song by UUID: play_song <uuid>"""
        if not args:
            print("Usage: play_song <uuid>")
            return
        song_info = self.client.show_song(args)
        self.print_song_details(song_info)
        file_path = song_info.get("file_path")
        if not file_path:
            print("File path not found for the song")
            return
        file_content = self.client.get_file(file_path)
        temp_file_path = "temp_song.mp3"
        temp_art_path = "temp_art.jpg"

        with open(temp_file_path, "wb") as f:
            f.write(file_content)

        art_path = song_info.get("song_art_path")
        if art_path:
            art_content = self.client.get_file(art_path)
            with open(temp_art_path, "wb") as f:
                f.write(art_content)
            subprocess.run(["mpv", temp_file_path, "--image-display-duration=inf", f"--external-file={temp_art_path}"])
            os.remove(temp_art_path)
        else:
            subprocess.run(["mpv", temp_file_path])

        os.remove(temp_file_path)

    def do_search(self, args):
        """Perform a fuzzy search: search <query>"""
        if not args:
            print("Usage: search <query>")
            return
        query = " ".join(args.split())
        query = query.replace("/", "%2F")
        result = self.client.search(query)
        self.print_search_results(result)

    def do_like_song(self, args):
        """Like a song by UUID: like_song <uuid>"""
        if not args:
            print("Usage: like_song <uuid>")
            return
        result = self.client.like_song(args)
        self.pretty_print_json(result)

    def do_unlike_song(self, args):
        """Unlike a song by UUID: unlike_song <uuid>"""
        if not args:
            print("Usage: unlike_song <uuid>")
            return
        result = self.client.unlike_song(args)
        self.pretty_print_json(result)

    def do_like_album(self, args):
        """Like an album by UUID: like_album <uuid>"""
        if not args:
            print("Usage: like_album <uuid>")
            return
        result = self.client.like_album(args)
        self.pretty_print_json(result)

    def do_unlike_album(self, args):
        """Unlike an album by UUID: unlike_album <uuid>"""
        if not args:
            print("Usage: unlike_album <uuid>")
            return
        result = self.client.unlike_album(args)
        self.pretty_print_json(result)

    def do_like_artist(self, args):
        """Like an artist by UUID: like_artist <uuid>"""
        if not args:
            print("Usage: like_artist <uuid>")
            return
        result = self.client.like_artist(args)
        self.pretty_print_json(result)

    def do_unlike_artist(self, args):
        """Unlike an artist by UUID: unlike_artist <uuid>"""
        if not args:
            print("Usage: unlike_artist <uuid>")
            return
        result = self.client.unlike_artist(args)
        self.pretty_print_json(result)

    def do_scan_directory(self, args):
        """Scan a directory for music files: scan_directory <directory>"""
        if not args:
            print("Usage: scan_directory <directory>")
            return
        result = self.client.scan(args)
        self.pretty_print_json(result)

    def do_exit(self):
        """Exit the console"""
        print("Goodbye!")
        return True

if __name__ == "__main__":
    console = MusicLibraryConsole()
    console.cmdloop()