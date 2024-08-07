import itertools
from rapidfuzz import fuzz
from mutagen.flac import FLAC
import os
import csv
import uuid
from musiclib_display import MusicLibraryDisplay
import re
from datetime import datetime
import subprocess
import tempfile
import shutil
import opencc
from multiprocessing import Pool, cpu_count
from functools import lru_cache


whitespace_re = re.compile(r'\s+')

class Event:
    def __init__(self, name):
        self.name = name
        self.uuid = str(uuid.uuid4())
        self.is_liked = False
        self.liked_time = None
        self.albums = []
        self.year = None

    def like(self):
        self.is_liked = True
        self.liked_time = datetime.now()
        print(f"Event {self.name} liked.")

    def unlike(self):
        self.is_liked = False
        print(f"Event {self.name} unliked.")
        
    def update_year(self):
        if self.year is None:
            for album in self.albums:
                if album.year is not None:
                    self.year = album.year

class Artist:
    def __init__(self, name):
        self.name = name
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
        self.event = {'uuid': None, 'name': None}

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
    
    def update_event(self):
        if not self.event['uuid'] and self.songs:  # 修改这里
            for song in self.songs:
                if song.event['uuid']:  # 修改这里
                    self.event = song.event  # 修改这里
                    break

class Song:
    def __init__(self, name, album, artists, file_path, track_number=1, disc_number=1, year=None, song_art_path=None, event=None):
        self.name = name
        self.uuid = str(uuid.uuid4())
        self.album = {'name': album.name, 'uuid': album.uuid}
        self.artists = [{'name': artist.name, 'uuid': artist.uuid} for artist in artists]
        self.file_path = file_path
        self.track_number = track_number
        self.disc_number = disc_number
        self.is_liked = False
        self.liked_time = None
        self.song_art_path = self.song_art_path = song_art_path or self.find_art_path(file_path)
        self.year = year
        self.event = {'uuid': event.uuid if event else None, 'name': event.name if event else None}

    @lru_cache(maxsize=None)
    def generate_possible_art_files(self):
        art_files = ['folder.jpg', 'cover.jpg', 'folder.png', 'cover.png', 
                    'folder.tif', 'cover.tif', 'folder.tiff', 'cover.tiff', 
                    'folder.jpeg', 'cover.jpeg']
        
        possible_files = set()
        for art_file in art_files:
            # 生成所有可能的大小写组合
            base_name, ext = os.path.splitext(art_file)
            for combo in itertools.product(*((char.lower(), char.upper()) for char in base_name)):
                possible_files.add(''.join(combo) + ext.lower())
        
        return possible_files

    def find_art_path(self, file_path):
        folder_path = os.path.dirname(file_path)
        possible_files = self.generate_possible_art_files()
        
        for art_file in possible_files:
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
        self.events = {}
        self.artists = {}
        self.albums = {}
        self.songs = {}
        self.display = MusicLibraryDisplay(self)
        self.graph = {}
        self.cc = opencc.OpenCC('t2s')
    
    def __getstate__(self):
        # 获取对象的状态，并移除不可序列化的部分
        state = self.__dict__.copy()
        state['cc'] = None  # 移除 OpenCC 对象
        return state

    def __setstate__(self, state):
        self.__dict__.update(state)
        self.cc = opencc.OpenCC('t2s')
        
    @lru_cache(maxsize=None)
    def normalize_name(self, name, for_search = False):
        normalized_name = self.cc.convert(name.strip().lower())
        normalized_name = normalized_name.translate(str.maketrans(
            "ァィゥェォャュョッーアイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワヲンヴヵヶ",
            "ぁぃぅぇぉゃゅょっーあいうえおかきくけこさしすせそたちつてとなにぬねのはひふへほまみむめもやゆよらりるれろわをんゔゕゖ"
        ))
        normalized_name = re.sub(r'\s+', '', normalized_name) if for_search else normalized_name
        return normalized_name

    @staticmethod
    @lru_cache(maxsize=None)
    def normalize_name_2(name, cc):
        normalized_name = cc.convert(name.strip().lower())
        normalized_name = normalized_name.translate(str.maketrans(
            "ァィゥェォャュョッーアイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワヲンヴヵヶ",
            "ぁぃぅぇぉゃゅょっーあいうえおかきくけこさしすせそたちつてとなにぬねのはひふへほまみむめもやゆよらりるれろわをんゔゕゖ"
        ))
        normalized_name = re.sub(r'\s+', '', normalized_name)  # 去除多余的空格
        return normalized_name

    @staticmethod
    @lru_cache(maxsize=None)
    def combined_score(attribute, query, cc):
        normalized_query = MusicLibrary.normalize_name_2(query, cc)
        normalized_attribute = MusicLibrary.normalize_name_2(attribute, cc)

        def get_match_score(normalized_query, normalized_attribute):
            return fuzz.ratio(normalized_query, normalized_attribute)

        match_score = get_match_score(normalized_query, normalized_attribute)
        length_score = max(0, (1 - abs(len(attribute) - len(query)) / len(query)) * 100)
        return match_score * 0.7 + length_score * 0.3

    @staticmethod
    def calculate_scores(items, attribute_getter, query, cc):
        with Pool(cpu_count()) as pool:
            scores = pool.starmap(
                MusicLibrary.combined_score,
                [(attribute_getter(item), query, cc) for item in items]
            )
        return scores
    
    @staticmethod
    def get_song_name(song):
        return song.name

    @staticmethod
    def get_album_name(album):
        return album.name

    @staticmethod
    def get_artist_name(artist):
        return artist.name

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
        normalized_name = self.normalize_name(name)
        for event in self.events.values():
            if self.normalize_name(event.name) == normalized_name:
                return event
        return None

    @lru_cache(maxsize=None)
    def find_artist_by_name(self, name):
        normalized_name = self.normalize_name(name)
        for artist in self.artists.values():
            if self.normalize_name(artist.name) == normalized_name:
                return artist
        return None

    def find_album_by_name(self, name):
        name = name.strip()
        for album in self.albums.values():
            if album.name == name:
                return album
        return None
    
    @lru_cache(maxsize=None)
    def find_album_by_name_artist_year(self, name, album_artist_names, year, album_artist_tag):
        album_artist_names_set = {self.normalize_name(album_artist_name) for album_artist_name in album_artist_names}
        if not album_artist_tag:
            album_artist_names = None
            album_artist_names_set = None
        for album in self.albums.values():
            album_artist_names_set_in_album = {self.normalize_name(artist.name) for artist in album.album_artists}
            if ((self.normalize_name(album.name) == self.normalize_name(name) or album.name is None or name is None) and
                (album.year == year or album.year is None or year is None) and
                (album_artist_names_set == album_artist_names_set_in_album or not album_artist_names or not album.album_artists)):
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
                if file.endswith(".flac"):
                    scanned_count += 1
                    if scanned_count % 250 == 0:
                        print(f"Scanned {scanned_count} files.")
                    file_path = os.path.join(root, file)
                    self.scan_file(file_path)
        self.graph = self.build_graph()
        self.auto_merge()

    @lru_cache(maxsize=None)
    def scan_file(self, file_path):
        try:
            album_artist_tag = True
            id3_tags = self.extract_id3_tags(file_path)
            song_name = id3_tags['title'].strip()
            album_name = id3_tags['album'].strip()
            artist_names = [name.strip() for name in id3_tags['artists']]
            album_artist_names = [name.strip() for name in id3_tags.get('album_artists', artist_names)]
            if album_artist_names == artist_names:
                album_artist_tag = False
            track_number = id3_tags['track_number']
            disc_number = id3_tags['disc_number']
            year = id3_tags['year']
            event_names = id3_tags['event']

            if isinstance(event_names, list):
                event_name = event_names[0].strip() if event_names else None
            else:
                event_name = event_names.strip() if event_names else None
            
            album_artist_names_tuple = tuple(album_artist_names)
            album = self.find_album_by_name_artist_year(album_name, album_artist_names_tuple, year, album_artist_tag)
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

            song_artists = []
            for artist_name in artist_names:
                artist = self.find_artist_by_name(artist_name)
                if not artist:
                    artist = Artist(artist_name)
                    self.find_artist_by_name.cache_clear()
                    self.add_artist(artist)
                song_artists.append(artist)

            song_art_path = album.album_art_path if album.album_art_path else None

            # Handling event
            event = None
            if event_name:
                event = self.find_event_by_name(event_name)
                if not event:
                    print(f"Adding new event {event_name}.")
                    event = Event(event_name)
                    self.find_event_by_name.cache_clear()
                    self.add_event(event)
                if album not in event.albums:
                    event.albums.append(album)
                if album.event is None:
                    album.event = {'uuid': event.uuid, 'name': event.name}

            song = Song(song_name, album, song_artists, file_path, track_number, disc_number, year, song_art_path, event)
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
            album.update_event()
            
            if event:
                event.update_year()

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
            raise

    def extract_id3_tags(self, file_path):
        file_extension = os.path.splitext(file_path)[1].lower()
        
        if file_extension == '.flac':
            return self.extract_flac_tags(file_path)
        else:
            print(f"Unsupported file format: {file_extension}")
            return {
                'title': 'Unknown Title',
                'album': 'Unknown Album',
                'artists': ['Unknown Artist'],
                'track_number': 1,
                'disc_number': 1,
                'event': 'Unknow Event',
                'year': None
            }

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
                'event': 'Unknow Event',
                'year': None
            }

        title = audio.get('title', ['Unknown Title'])[0]
        album = audio.get('album', ['Unknown Album'])[0]
        artists = audio.get('artist', ['Unknown Artist'])
        event = audio.get('event', ['Unknow Event'])
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

        return self.parse_artists(title, album, artists, track_number, disc_number, album_artists, year, event)


    def parse_artists(self, title, album, artists, track_number=1, disc_number=1, album_artists=None, year=None, event = None):
        delimiters = ['/', '／', '&', '＆', ' x ', ';', '；', ',', '，', '×', '　', '、']
        delimiters = tuple(delimiters)
        ignore = ['cool&create', 'Factory Noise&AG', 'Sing, R. Sing!']
        ignore_normalized = [self.normalize_name(item) for item in ignore]
        parsed_artists = []

        for artist in artists:
            parts = []

            while artist:
                artist_normalized = self.normalize_name(artist)
                for ignored in ignore_normalized:
                    ignored_index = artist_normalized.find(ignored)
                    if ignored_index != -1:
                        pre_ignored = artist[:ignored_index].strip()
                        ignored_part = artist[ignored_index:ignored_index + len(ignored)]
                        post_ignored = artist[ignored_index + len(ignored):].strip()

                        if pre_ignored:
                            parts.extend(self.split_and_clean(pre_ignored, delimiters))

                        parts.append(ignored_part)
                        artist = post_ignored
                        break
                else:
                    parts.extend(self.split_and_clean(artist, delimiters))
                    break

            parsed_artists.extend(parts)

        if album_artists:
            parsed_album_artists = []
            for artist in album_artists:
                parts = []

                while artist:
                    artist_normalized = self.normalize_name(artist)
                    for ignored in ignore_normalized:
                        ignored_index = artist_normalized.find(ignored)
                        if ignored_index != -1:
                            pre_ignored = artist[:ignored_index].strip()
                            ignored_part = artist[ignored_index:ignored_index + len(ignored)]
                            post_ignored = artist[ignored_index + len(ignored):].strip()

                            if pre_ignored:
                                parts.extend(self.split_and_clean(pre_ignored, delimiters))

                            parts.append(ignored_part)
                            artist = post_ignored
                            break
                    else:
                        parts.extend(self.split_and_clean(artist, delimiters))
                        break

                parsed_album_artists.extend(parts)
        else:
            parsed_album_artists = parsed_artists

        return {
            'title': title,
            'album': album,
            'artists': parsed_artists,
            'album_artists': parsed_album_artists,
            'track_number': track_number,
            'disc_number': disc_number,
            'year': year,
            'event': event
        }
    
    @lru_cache(maxsize=None)
    def split_and_clean(self, text, delimiters):
        temp_artists = [text]
        for delimiter in delimiters:
            new_temp_artists = []
            for temp_artist in temp_artists:
                new_temp_artists.extend([sub_artist.strip() for sub_artist in temp_artist.split(delimiter) if sub_artist.strip()])
            temp_artists = new_temp_artists
        return temp_artists
    
    def auto_merge(self):
        file_path = os.path.join(os.getcwd(), 'artist_alias.csv')
        
        if not os.path.isfile(file_path):
            print("File artist_alias.csv not found in the current directory.")
            return

        with open(file_path, newline='', encoding='utf-8-sig') as csvfile:
            reader = csv.reader(csvfile)
            for row in reader:
                print(f"Processing row: {row}")
                if len(row) < 2:
                    print(f"Skipping row with insufficient data: {row}")
                    continue
                
                primary_artist_name = row[0].strip()
                primary_artist = self.find_artist_by_name(primary_artist_name)
                
                if not primary_artist:
                    print(f"Primary artist '{primary_artist_name}' not found.")
                    continue

                for alias in row[1:]:
                    alias_name = alias.strip()
                    alias_artist = self.find_artist_by_name(alias_name)
                    
                    if not alias_artist:
                        print(f"Alias artist '{alias_name}' not found.")
                        continue
                    
                    if primary_artist_name.lower() != alias_name.lower():
                        print(f"Merging alias artist '{alias_name}' into primary artist '{primary_artist_name}'")
                        self.merge_artist_by_name(primary_artist_name, alias_name)
                        print(f"Successfully merged '{alias_name}' into '{primary_artist_name}'")

        for event in self.events.values():
            for album in event.albums:
                if album.year is None:
                    print(f"Setting year for album {album.name} and its songs to {event.year}")
                    album.year = event.year
                    for song in album.songs:
                        song.year = event.year

        for album in self.albums.values():
            if album.album_art_path:
                for song in album.songs:
                    if not song.song_art_path:
                        print(f"Setting song art for song {song.name} to album art {album.album_art_path}")
                        song.song_art_path = album.album_art_path


    def merge_artist_by_uuid(self, uuid1, uuid2):
        if uuid1 not in self.artists or uuid2 not in self.artists:
            print(f"One or both artist UUIDs not found: uuid1={uuid1}, uuid2={uuid2}")
            return
        
        artist1 = self.artists[uuid1]
        artist2 = self.artists[uuid2]

        # Update albums
        for album in self.albums.values():
            if artist2 in album.album_artists:
                album.album_artists.remove(artist2)
                album.album_artists.add(artist1)
                if not album.album_art_path and artist1.artist_art_path:
                    album.album_art_path = artist1.artist_art_path

        # Update songs
        for song in self.songs.values():
            artist_updated = False
            for artist in song.artists:
                if artist['uuid'] == uuid2:
                    artist['uuid'] = uuid1
                    artist['name'] = artist1.name
                    artist_updated = True

            if artist_updated:
                if not song.song_art_path and artist1.artist_art_path:
                    song.song_art_path = artist1.artist_art_path
                if song.album['uuid'] in self.albums:
                    album = self.albums[song.album['uuid']]
                    for album_artist in album.album_artists:
                        if album_artist.uuid == uuid2:
                            album_artist.uuid = uuid1
                            album_artist.name = artist1.name

        # Update graph
        if uuid2 in self.graph:
            if uuid1 not in self.graph:
                self.graph[uuid1] = {}
            for neighbor, data in self.graph[uuid2].items():
                if neighbor not in self.graph[uuid1]:
                    self.graph[uuid1][neighbor] = data
                else:
                    self.graph[uuid1][neighbor]['strength'] += data['strength']
                    self.graph[uuid1][neighbor]['details'].update(data['details'])
                if neighbor in self.graph:
                    if uuid2 in self.graph[neighbor]:
                        del self.graph[neighbor][uuid2]
                    if uuid1 not in self.graph[neighbor]:
                        self.graph[neighbor][uuid1] = data
                    else:
                        self.graph[neighbor][uuid1]['strength'] += data['strength']
                        self.graph[neighbor][uuid1]['details'].update(data['details'])
            del self.graph[uuid2]

        del self.artists[uuid2]
        print(f"Artist {uuid2} merged into {uuid1}.")

    def merge_artist_by_name(self, name1, name2):
        artist1 = self.find_artist_by_name(name1)
        artist2 = self.find_artist_by_name(name2)
        
        if not artist1 or not artist2:
            print(f"One or both artists not found: name1={name1}, name2={name2}")
            return

        uuid1 = artist1.uuid
        uuid2 = artist2.uuid

        self.merge_artist_by_uuid(uuid1, uuid2)
        print(f"Artist {name2} merged into {name1}.")

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
    
    def search_event(self, name, year):
        return self.find_event_by_name_year(name, year)

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
    
    @lru_cache(maxsize=None)
    def search(self, query):
        normalized_query = self.normalize_name(query, True)
        print(normalized_query)
        
        song_scores = MusicLibrary.calculate_scores(self.songs.values(), MusicLibrary.get_song_name, normalized_query, self.cc)
        album_scores = MusicLibrary.calculate_scores(self.albums.values(), MusicLibrary.get_album_name, normalized_query, self.cc)
        artist_scores = MusicLibrary.calculate_scores(self.artists.values(), MusicLibrary.get_artist_name, normalized_query, self.cc)

        matched_songs = [(song, score) for song, score in zip(self.songs.values(), song_scores) if score > 50]
        matched_albums = [(album, score) for album, score in zip(self.albums.values(), album_scores) if score > 50]
        matched_artists = [(artist, score) for artist, score in zip(self.artists.values(), artist_scores) if score > 50]

        # Sort by score in descending order
        matched_songs.sort(key=lambda x: x[1], reverse=True)
        matched_albums.sort(key=lambda x: x[1], reverse=True)
        matched_artists.sort(key=lambda x: x[1], reverse=True)

        # Limit the results
        limited_songs = [song for song, score in matched_songs[:200]]
        limited_albums = [album for album, score in matched_albums[:50]]
        limited_artists = [artist for artist, score in matched_artists[:50]]
        
        return {
            'songs': limited_songs,
            'albums': limited_albums,
            'artists': limited_artists
        }
    
    def build_graph(self):
        graph = {}
        
        def add_edge(a, b, relation):
            if a not in graph:
                graph[a] = {}
            if b not in graph:
                graph[b] = {}
            if b not in graph[a]:
                graph[a][b] = {"strength": 0, "details": set()}
            if a not in graph[b]:
                graph[b][a] = {"strength": 0, "details": set()}
            graph[a][b]["strength"] += 1
            graph[a][b]["details"].add(relation)
            graph[b][a]["strength"] += 1
            graph[b][a]["details"].add(relation)

        # 遍历所有歌曲
        for song in self.songs.values():
            artist_uuids = [artist['uuid'] for artist in song.artists if artist['name'].lower() != 'various artists']
            for i in range(len(artist_uuids)):
                for j in range(i + 1, len(artist_uuids)):
                    a, b = artist_uuids[i], artist_uuids[j]
                    add_edge(a, b, f"same song: {song.name} ({song.uuid})")
                    
        # 遍历所有专辑
        for album in self.albums.values():
            album_artist_uuids = [artist.uuid for artist in album.album_artists if artist.name.lower() != 'various artists']
            
            # 专辑艺术家之间的关系
            for i in range(len(album_artist_uuids)):
                for j in range(i + 1, len(album_artist_uuids)):
                    a, b = album_artist_uuids[i], album_artist_uuids[j]
                    add_edge(a, b, f"same album: {album.name} ({album.uuid})")
            
            # 专辑中的歌曲艺术家之间的关系
            for song in album.songs:
                song_artist_uuids = [artist['uuid'] for artist in song.artists if artist['name'].lower() != 'various artists']
                for i in range(len(song_artist_uuids)):
                    for j in range(i + 1, len(song_artist_uuids)):
                        a, b = song_artist_uuids[i], song_artist_uuids[j]
                        add_edge(a, b, f"same song: {song.name} ({song.uuid})")
                
                # 专辑艺术家与歌曲艺术家之间的关系
                for album_artist_uuid in album_artist_uuids:
                    for song_artist_uuid in song_artist_uuids:
                        if album_artist_uuid != song_artist_uuid:
                            add_edge(album_artist_uuid, song_artist_uuid, f"album artist with song artist: {album.name} ({album.uuid})")
        return graph

    def find_relation(self, artist_uuid, graph):
        if artist_uuid not in self.artists:
            return {"nodes": [], "edges": []}

        nodes = [{"uuid": artist_uuid, "name": self.artists[artist_uuid].name}]
        edges = []
        visited = set([artist_uuid])

        def add_node_and_edge(source, target, data):
            if target not in visited:
                nodes.append({"uuid": target, "name": self.artists[target].name})
                visited.add(target)
            edges.append({
                "source": source,
                "target": target,
                "strength": data["strength"],
                "details": list(data["details"])
            })

        for neighbor, data in graph.get(artist_uuid, {}).items():
            add_node_and_edge(artist_uuid, neighbor, data)
        for neighbor in graph:
            if artist_uuid in graph[neighbor]:
                data = graph[neighbor][artist_uuid]
                add_node_and_edge(neighbor, artist_uuid, data)

        return {"nodes": nodes, "edges": edges}
    
    def show_relation(self, artist_uuid, layer):
        graph = self.graph

        if artist_uuid == "show_all":
            nodes = []
            edges = []
            for a, neighbors in graph.items():
                if a not in self.artists:
                    continue
                nodes.append({"uuid": a, "name": self.artists[a].name})
                for b, data in neighbors.items():
                    if b not in self.artists:
                        continue
                    nodes.append({"uuid": b, "name": self.artists[b].name})
                    edges.append({
                        "source": a,
                        "target": b,
                        "strength": data["strength"],
                        "details": list(data["details"])
                    })
            unique_nodes = {node['uuid']: node for node in nodes}
            unique_edges = {tuple(sorted([edge['source'], edge['target']])): edge for edge in edges}
            return {"nodes": list(unique_nodes.values()), "edges": list(unique_edges.values())}

        if layer == 1:
            return self.find_relation(artist_uuid, graph)

        if layer < 1:
            return {"nodes": [{"uuid": artist_uuid, "name": self.artists[artist_uuid].name}], "edges": []}

        all_nodes = [{"uuid": artist_uuid, "name": self.artists[artist_uuid].name}]
        all_edges = []
        visited = {artist_uuid}
        queue = [(artist_uuid, 0)]

        while queue:
            current_artist_uuid, current_layer = queue.pop(0)
            if current_layer >= layer:
                continue

            current_relations = self.find_relation(current_artist_uuid, graph)
            for node in current_relations["nodes"]:
                if node["uuid"] not in visited:
                    visited.add(node["uuid"])
                    all_nodes.append(node)
                    if current_layer + 1 < layer:
                        queue.append((node["uuid"], current_layer + 1))

            for edge in current_relations["edges"]:
                all_edges.append(edge)

        # 去重节点和边
        unique_nodes = {node['uuid']: node for node in all_nodes}
        unique_edges = {tuple(sorted([edge['source'], edge['target']])): edge for edge in all_edges}

        return {"nodes": list(unique_nodes.values()), "edges": list(unique_edges.values())}

# todo: combine artist = add_artist & merge_artist, migrate artist_art!


if __name__ == "__main__":
    library = MusicLibrary()
    library.scan('/Users/a1/other')

    library.display.show_library()