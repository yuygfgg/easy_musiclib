from mutagen.easyid3 import EasyID3
from mutagen.flac import FLAC
import os
import uuid

class Artist:
    def __init__(self, name):
        self.name = name.lower()
        self.uuid = str(uuid.uuid4())
        self.is_liked = False
        self.artist_art_path = ""

    def like(self):
        self.is_liked = True
        print(f"Artist {self.name} liked.")

    def unlike(self):
        self.is_liked = False
        print(f"Artist {self.name} unliked.")

class Album:
    def __init__(self, name):
        self.name = name
        self.uuid = str(uuid.uuid4())
        self.album_artists = set()
        self.songs = []
        self.is_liked = False
        self.album_art_path = ""

    def like(self):
        self.is_liked = True
        print(f"Album {self.name} liked.")

    def unlike(self):
        self.is_liked = False
        print(f"Album {self.name} unliked.")

class Song:
    def __init__(self, name, album, artists, file_path, track_number=1, disc_number=1):
        self.name = name
        self.uuid = str(uuid.uuid4())
        self.album = {'name': album.name, 'uuid': album.uuid}
        self.artists = [{'name': artist.name.lower(), 'uuid': artist.uuid} for artist in artists]
        self.file_path = file_path
        self.track_number = track_number
        self.disc_number = disc_number
        self.is_liked = False
        self.song_art_path = self.find_art_path(file_path)

    def find_art_path(self, file_path):
        folder_path = os.path.dirname(file_path)
        for art_file in ['folder.jpg', 'cover.jpg']:
            art_path = os.path.join(folder_path, art_file)
            if os.path.isfile(art_path):
                return art_path
        return ""

    def like(self):
        self.is_liked = True
        print(f"Song {self.name} liked.")

    def unlike(self):
        self.is_liked = False
        print(f"Song {self.name} unliked.")

class MusicLibrary:
    def __init__(self):
        self.artists = {}
        self.albums = {}
        self.songs = {}

    def add_song(self, song):
        self.songs[song.uuid] = song

    def add_album(self, album):
        self.albums[album.uuid] = album

    def add_artist(self, artist):
        self.artists[artist.uuid] = artist

    def find_artist_by_name(self, name):
        name = name.lower()
        for artist in self.artists.values():
            if artist.name == name:
                return artist
        return None

    def find_album_by_name(self, name):
        for album in self.albums.values():
            if album.name == name:
                return album
        return None

    def find_song_by_name(self, name):
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

    def scan(self, directory):
        for root, dirs, files in os.walk(directory):
            for file in files:
                if file.endswith(".mp3") or file.endswith(".flac"):
                    file_path = os.path.join(root, file)
                    id3_tags = self.extract_id3_tags(file_path)
                    song_name = id3_tags['title']
                    album_name = id3_tags['album']
                    artist_names = id3_tags['artists']
                    album_artist_names = id3_tags.get('album_artists', artist_names)
                    track_number = id3_tags['track_number']
                    disc_number = id3_tags['disc_number']

                    album = self.find_album_by_name(album_name)
                    if not album:
                        album = Album(album_name)
                        self.add_album(album)

                    for artist_name in album_artist_names:
                        artist = self.find_artist_by_name(artist_name)
                        if not artist:
                            artist = Artist(artist_name)
                            self.add_artist(artist)
                        album.album_artists.add(artist)

                    song_artists = []
                    for artist_name in artist_names:
                        artist = self.find_artist_by_name(artist_name)
                        if not artist:
                            artist = Artist(artist_name)
                            self.add_artist(artist)
                        song_artists.append(artist)

                    song = Song(song_name, album, song_artists, file_path, track_number, disc_number)
                    self.add_song(song)
                    album.songs.append(song)

                    # Set album art path if not already set
                    if not album.album_art_path:
                        album.album_art_path = song.song_art_path

                    # Set artist art path if not already set
                    for artist in album.album_artists:
                        if not artist.artist_art_path:
                            artist.artist_art_path = album.album_art_path

    def extract_id3_tags(self, file_path):
        file_extension = os.path.splitext(file_path)[1].lower()

        if file_extension == '.mp3':
            return self.extract_mp3_tags(file_path)
        elif file_extension == '.flac':
            return self.extract_flac_tags(file_path)
        else:
            print(f"Unsupported file format: {file_extension}")
            return {
                'title': 'Unknown Title',
                'album': 'Unknown Album',
                'artists': ['Unknown Artist'],
                'track_number': 1,
                'disc_number': 1
            }

    def extract_mp3_tags(self, file_path):
        try:
            audio = EasyID3(file_path)
        except Exception as e:
            print(f"Error reading ID3 tags from {file_path}: {e}")
            return {
                'title': 'Unknown Title',
                'album': 'Unknown Album',
                'artists': ['Unknown Artist'],
                'track_number': 1,
                'disc_number': 1
            }

        title = audio.get('title', ['Unknown Title'])[0]
        album = audio.get('album', ['Unknown Album'])[0]
        artists = audio.get('artist', ['Unknown Artist'])
        if isinstance(artists, str):
            artists = [artists]
            
        track_number = int(audio.get('tracknumber', ['1'])[0].split('/')[0])
        disc_number = int(audio.get('discnumber', ['1'])[0].split('/')[0])

        return self.parse_artists(title, album, artists, track_number, disc_number)

    def extract_flac_tags(self, file_path):
        try:
            audio = FLAC(file_path)
        except Exception as e:
            print(f"Error reading FLAC tags from {file_path}: {e}")
            return {
                'title': 'Unknown Title',
                'album': 'Unknown Album',
                'artists': ['Unknown Artist'],
                'track_number': 1,
                'disc_number': 1
            }

        title = audio.get('title', ['Unknown Title'])[0]
        album = audio.get('album', ['Unknown Album'])[0]
        artists = audio.get('artist', ['Unknown Artist'])
        if isinstance(artists, str):
            artists = [artists]

        album_artists = audio.get('albumartist', artists)
        if isinstance(album_artists, str):
            album_artists = [album_artists]  # Ensure it's always a list

        track_number = int(audio.get('tracknumber', ['1'])[0].split('/')[0])
        disc_number = int(audio.get('discnumber', ['1'])[0].split('/')[0])

        return self.parse_artists(title, album, artists, track_number, disc_number, album_artists)

    def parse_artists(self, title, album, artists, track_number=1, disc_number=1, album_artists=None):
        delimiters = ['/', '&', ' x ', ';', '；']
        parsed_artists = []
        for artist in artists:
            for delimiter in delimiters:
                if delimiter in artist:
                    parsed_artists.extend([a.strip().lower() for a in artist.split(delimiter)])
                    break
            else:
                parsed_artists.append(artist.lower())

        if album_artists:
            parsed_album_artists = []
            for artist in album_artists:
                for delimiter in delimiters:
                    if delimiter in artist:
                        parsed_album_artists.extend([a.strip().lower() for a in artist.split(delimiter)])
                        break
                else:
                    parsed_album_artists.append(artist.lower())
        else:
            parsed_album_artists = parsed_artists
        return {
            'title': title,
            'album': album,
            'artists': parsed_artists,
            'album_artists': parsed_album_artists,
            'track_number': track_number,
            'disc_number': disc_number
        }

    def search_song(self, name):
        return self.find_song_by_name(name)

    def search_album(self, name):
        return self.find_album_by_name(name)

    def search_artist(self, name):
        return self.find_artist_by_name(name)

    def show_songinfo(self, song):
        print("------------start song information------------")
        print(f"Song: {song.name}")
        print(f"UUID: {song.uuid}")
        print(f"Album: {song.album['name']} (UUID: {song.album['uuid']})")
        artist_info = ', '.join([f"{artist['name']} (UUID: {artist['uuid']})" for artist in song.artists])
        print(f"Artists: {artist_info}")
        print(f"File Path: {song.file_path}")
        print(f"Track Number: {song.track_number}")
        print(f"Disc Number: {song.disc_number}")
        print(f"Song Art Path: {song.song_art_path}")
        print(f"Isliked: {song.is_liked}")
        print("------------end song information------------")

    def show_albuminfo(self, album):
        print("------------start album information------------")
        print(f"Album: {album.name}")
        print(f"UUID: {album.uuid}")
        for artist in album.album_artists:
            print(f"Album artist: {artist.name} ({artist.uuid})")
        print("Songs:")
        for song in album.songs:
            print(f"  - {song.name} (UUID: {song.uuid}, Track: {song.track_number}, Disc: {song.disc_number})")
        print(f"Album Art Path: {album.album_art_path}")
        print(f"Isliked: {album.is_liked}")
        print("------------end album information------------")

    def show_artistinfo(self, artist):
        print("------------start artist information------------")
        if isinstance(artist, (set, list)):
            for single_artist in artist:
                self._show_single_artistinfo(single_artist)
        else:
            self._show_single_artistinfo(artist)
        print("------------end artist information------------")

    def _show_single_artistinfo(self, artist):
        print(f"Artist: {artist.name}")
        print(f"UUID: {artist.uuid}")
        print("Albums:")
        for album in self.albums.values():
            if artist in album.album_artists:
                print(f"  - {album.name} (UUID: {album.uuid})")
        print("Songs:")
        for song in self.songs.values():
            if artist.uuid in [a['uuid'] for a in song.artists]:
                print(f"  - {song.name} (UUID: {song.uuid}, Track: {song.track_number}, Disc: {song.disc_number})")
        print(f"Artist Art Path: {artist.artist_art_path}")
        print(f"Isliked: {artist.is_liked}")

    def show_library(self):
        print("Artists:")
        for artist in self.artists.values():
            self.show_artistinfo(artist)
        print("\nAlbums:")
        for album in self.albums.values():
            self.show_albuminfo(album)
        print("\nSongs:")
        for song in self.songs.values():
            self.show_songinfo(song)

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
    
    def show_liked_songs(self):
        liked_songs = [song for song in self.songs.values() if song.is_liked]
        for song in liked_songs:
            print(f"{song.name} (UUID: {song.uuid}) Disc_number: {song.disc_number}")

    def show_liked_artists(self):
        liked_artists = [artist for artist in self.artists.values() if artist.is_liked]
        for artist in liked_artists:
            print(f"{artist.name} (UUID: {artist.uuid})")

    def show_liked_albums(self):
        liked_albums = [album for album in self.albums.values() if album.is_liked]
        for album in liked_albums:
            print(f"{album.name} (UUID: {album.uuid})")
    
    def search(self, query):
        import re
        pattern = re.compile(re.escape(query), re.IGNORECASE)
        
        matched_songs = [song for song in self.songs.values() if pattern.search(song.name)]
        matched_albums = [album for album in self.albums.values() if pattern.search(album.name)]
        matched_artists = [artist for artist in self.artists.values() if pattern.search(artist.name)]

        return {
            'songs': matched_songs,
            'albums': matched_albums,
            'artists': matched_artists
        }

if __name__ == "__main__":
    library = MusicLibrary()
    library.scan('/Users/a1/other')

    # 展示整个库
    library.show_library()