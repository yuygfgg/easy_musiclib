from rapidfuzz import process, fuzz
from mutagen.easyid3 import EasyID3
from mutagen.flac import FLAC
import os
import uuid
from musiclib_display import MusicLibraryDisplay
import re
from datetime import datetime
import subprocess
import tempfile
import shutil

class Artist:
    def __init__(self, name):
        self.name = name.lower()
        self.uuid = str(uuid.uuid4())
        self.is_liked = False
        self.liked_time = None
        self.artist_art_path = ""

    def like(self):
        self.is_liked = True
        self.liked_time = datetime.now()
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
        self.liked_time = None
        self.album_art_path = ""
        self.year = None

    def like(self):
        self.is_liked = True
        self.liked_time = datetime.now()
        print(f"Album {self.name} liked.")

    def unlike(self):
        self.is_liked = False
        print(f"Album {self.name} unliked.")

    def update_year(self):
        if not self.year and self.songs:
            for song in self.songs:
                if song.year:
                    self.year = song.year
                    break

class Song:
    def __init__(self, name, album, artists, file_path, track_number=1, disc_number=1, year=None):
        self.name = name
        self.uuid = str(uuid.uuid4())
        self.album = {'name': album.name, 'uuid': album.uuid}
        self.artists = [{'name': artist.name.lower(), 'uuid': artist.uuid} for artist in artists]
        self.file_path = file_path
        self.track_number = track_number
        self.disc_number = disc_number
        self.is_liked = False
        self.liked_time = None
        self.song_art_path = self.find_art_path(file_path)
        self.year = year

    def find_art_path(self, file_path):
        folder_path = os.path.dirname(file_path)
        for art_file in ['folder.jpg', 'cover.jpg', 'folder.png', 'cover.png', 'folder.tif', 'cover.tif', 'folder.tiff', 'cover.tiff', 'folder.jpeg', 'cover.jpeg']:
            art_path = os.path.join(folder_path, art_file)
            if os.path.isfile(art_path):
                return art_path
        return ""

    def like(self):
        self.is_liked = True
        self.liked_time = datetime.now()
        print(f"Song {self.name} liked.")

    def unlike(self):
        self.is_liked = False
        print(f"Song {self.name} unliked.")

class MusicLibrary:
    def __init__(self):
        self.artists = {}
        self.albums = {}
        self.songs = {}
        self.display = MusicLibraryDisplay(self)

    def add_song(self, song):
        self.songs[song.uuid] = song

    def add_album(self, album):
        self.albums[album.uuid] = album

    def add_artist(self, artist):
        self.artists[artist.uuid] = artist

    def find_artist_by_name(self, name):
        name = name.strip()
        name = name.lower()
        for artist in self.artists.values():
            if artist.name == name:
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
    
    def scan(self, directory):
        scanned_count = 0
        for root, dirs, files in os.walk(directory):
            for file in files:
                if file.endswith(".mp3") or file.endswith(".flac"):
                    scanned_count += 1
                    if scanned_count % 250 == 0:
                        print(f"Scanned {scanned_count} files.")
                    try:
                        file_path = os.path.join(root, file)
                        id3_tags = self.extract_id3_tags(file_path)
                        song_name = id3_tags['title'].strip()
                        album_name = id3_tags['album'].strip()
                        artist_names = [name.strip() for name in id3_tags['artists']]
                        album_artist_names = [name.strip() for name in id3_tags.get('album_artists', artist_names)]
                        track_number = id3_tags['track_number']
                        disc_number = id3_tags['disc_number']
                        year = id3_tags['year']

                        album = self.find_album_by_name(album_name)
                        if not album:
                            print(f"Adding new album {album_name} because it doesn't exist now.")
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

                        song = Song(song_name, album, song_artists, file_path, track_number, disc_number, year)
                        self.add_song(song)
                        album.songs.append(song)

                        if not album.album_art_path:
                            album.album_art_path = song.song_art_path
                            if not album.album_art_path:
                                album.album_art_path = self.extract_embedded_art(song.file_path)
                                if album.album_art_path:
                                    for s in album.songs:
                                        if not s.song_art_path:
                                            s.song_art_path = album.album_art_path

                        album.update_year()

                        for artist in album.album_artists:
                            if not artist.artist_art_path:
                                artist.artist_art_path = album.album_art_path

                        for artist in song_artists:
                            if not artist.artist_art_path:
                                artist.artist_art_path = album.album_art_path
                                if not artist.artist_art_path:
                                    artist.artist_art_path = song.song_art_path

                    except Exception as e:
                        print(f"Error processing file {file_path}: {e}")
                        continue



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
                'disc_number': 1,
                'year': None
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
                'disc_number': 1,
                'year': None
            }

        title = audio.get('title', ['Unknown Title'])[0]
        album = audio.get('album', ['Unknown Album'])[0]
        artists = audio.get('artist', ['Unknown Artist'])
        if isinstance(artists, str):
            artists = [artists]
            
        track_number = int(audio.get('tracknumber', ['1'])[0].split('/')[0])
        disc_number = int(audio.get('discnumber', ['1'])[0].split('/')[0])

        year = audio.get('date', [None])[0] or audio.get('year', [None])[0]
        year = self.extract_year(year)

        return self.parse_artists(title, album, artists, track_number, disc_number, year=year)

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
                'disc_number': 1,
                'year': None
            }

        title = audio.get('title', ['Unknown Title'])[0]
        album = audio.get('album', ['Unknown Album'])[0]
        artists = audio.get('artist', ['Unknown Artist'])
        if isinstance(artists, str):
            artists = [artists]

        album_artists = audio.get('albumartist')
        if not album_artists:
            album_artists = audio.get('album artist', artists)
        if isinstance(album_artists, str):
            album_artists = [album_artists]  # Ensure it's always a list

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

        year = audio.get('date', [None])[0] or audio.get('year', [None])[0]
        year = self.extract_year(year)

        return self.parse_artists(title, album, artists, track_number, disc_number, album_artists, year)

    def parse_artists(self, title, album, artists, track_number=1, disc_number=1, album_artists=None, year=None):
        delimiters = ['/', '／', '&', ' x ', ';', '；', ',', '，', '×', '　', '、']
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
            'disc_number': disc_number,
            'year': year
        }

    def extract_year(self, date_string):
        if date_string:
            match = re.search(r'\b(\d{4})\b', date_string)
            if match:
                return int(match.group(1))
        return None

    def extract_embedded_art(self, file_path):
        try:
            with tempfile.TemporaryDirectory() as temp_dir:
                art_path = os.path.join(temp_dir, 'folder.jpg')

                ffmpeg_command = [
                    'ffmpeg', '-i', file_path,
                    '-an', '-vcodec', 'mjpeg', '-update', '1', '-y', art_path
                ]

                subprocess.run(ffmpeg_command, check=True, shell=False)

                if os.path.isfile(art_path):
                    final_art_path = os.path.join(os.path.dirname(file_path), 'folder.jpg')
                    shutil.copyfile(art_path, final_art_path)
                    return final_art_path
        except subprocess.CalledProcessError as e:
            print(f"Error extracting embedded art from {file_path}: {e}")
        except Exception as e:
            print(f"Unexpected error: {e}")
        return ""
    
    def search_song(self, name):
        return self.find_song_by_name(name)

    def search_album(self, name):
        return self.find_album_by_name(name)

    def search_artist(self, name):
        return self.find_artist_by_name(name)

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

    library.display.show_library()