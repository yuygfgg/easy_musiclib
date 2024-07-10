from rapidfuzz import process, fuzz
from mutagen.easyid3 import EasyID3
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
        self.graph = {}

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
        self.graph = self.build_graph()
        self.auto_merge()



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
        delimiters = ['/', '／', '&', '＆', ' x ', ';', '；', ',', '，', '×', '　', '、']
        ignore = ['cool&create']
        ignore_lower = [item.lower() for item in ignore]
        parsed_artists = []

        for artist in artists:
            artist_lower = artist.lower()
            parts = []
            
            while artist_lower:
                for ignored in ignore_lower:
                    ignored_index = artist_lower.find(ignored)
                    if ignored_index != -1:
                        pre_ignored = artist[:ignored_index].strip()
                        ignored_part = artist[ignored_index:ignored_index + len(ignored)]
                        post_ignored = artist[ignored_index + len(ignored):].strip()

                        if pre_ignored:
                            parts.extend(self.split_and_clean(pre_ignored, delimiters))
                        
                        parts.append(ignored_part)
                        artist = post_ignored
                        artist_lower = artist.lower()
                        break
                else:
                    parts.extend(self.split_and_clean(artist, delimiters))
                    break
            
            parsed_artists.extend(parts)

        if album_artists:
            parsed_album_artists = []
            for artist in album_artists:
                artist_lower = artist.lower()
                parts = []
                
                while artist_lower:
                    for ignored in ignore_lower:
                        ignored_index = artist_lower.find(ignored)
                        if ignored_index != -1:
                            pre_ignored = artist[:ignored_index].strip()
                            ignored_part = artist[ignored_index:ignored_index + len(ignored)]
                            post_ignored = artist[ignored_index + len(ignored):].strip()

                            if pre_ignored:
                                parts.extend(self.split_and_clean(pre_ignored, delimiters))
                            
                            parts.append(ignored_part)
                            artist = post_ignored
                            artist_lower = artist.lower()
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
            'year': year
        }

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
            print(f"File artist_alias.csv not found in the current directory.")
            return

        with open(file_path, newline='', encoding='utf-8') as csvfile:
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
            for artist in song.artists:
                if artist['uuid'] == uuid2:
                    artist['uuid'] = uuid1
                    artist['name'] = artist1.name
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


        
if __name__ == "__main__":
    library = MusicLibrary()
    library.scan('/Users/a1/other')

    library.display.show_library()