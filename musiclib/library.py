import importlib
import os
import csv
from functools import lru_cache

from musiclib import utils

from .tag_extracter import TagExtractor
from .models import Album, Artist, Event, Song
from .display import MusicLibraryDisplay
from .search import MusicLibrarySearch

class MusicLibrary:
    def __init__(self):
        self.events = {}
        self.artists = {}
        self.albums = {}
        self.songs = {}
        self.display = MusicLibraryDisplay(self)
        self.searcher = MusicLibrarySearch(self)
        self.graph = {}
        self.extractors = self.load_extractors()
    
    def __getstate__(self):
        # 获取对象的状态，并移除不可序列化的部分
        state = self.__dict__.copy()
        return state

    def __setstate__(self, state):
        self.__dict__.update(state)
    
    def load_extractors(self):
        extractors = {}
        for file in os.listdir('musiclib/tag_extracter'):
            if file.endswith('_extractor.py'):
                module_name = file[:-3]
                module = importlib.import_module(f'musiclib.tag_extracter.{module_name}')
                for attr in dir(module):
                    cls = getattr(module, attr)
                    if isinstance(cls, type) and issubclass(cls, TagExtractor) and cls is not TagExtractor:
                        # Extract the extension from the module name
                        extension = module_name.split('_')[0]
                        extractors[extension] = cls()
        return extractors

    def extract_tags(self, file_path):
        extension = os.path.splitext(file_path)[1][1:].lower()
        extractor = self.extractors.get(extension)
        if extractor:
            return extractor.extract(file_path)
        return None

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
        normalized_name = utils.normalize_name(name)
        for event in self.events.values():
            if utils.normalize_name(event.name) == normalized_name:
                return event
        return None

    @lru_cache(maxsize=None)
    def find_artist_by_name(self, name):
        normalized_name = utils.normalize_name(name)
        for artist in self.artists.values():
            if utils.normalize_name(artist.name) == normalized_name:
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
        album_artist_names_set = {utils.normalize_name(album_artist_name) for album_artist_name in album_artist_names}
        if not album_artist_tag:
            album_artist_names = None
            album_artist_names_set = None
        for album in self.albums.values():
            album_artist_names_set_in_album = {utils.normalize_name(artist.name) for artist in album.album_artists}
            if ((utils.normalize_name(album.name) == utils.normalize_name(name) or album.name is None or name is None) and
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
        existing_files = {song.file_path for song in self.songs.values()}
        
        for root, dirs, files in os.walk(directory):
            for file in files:
                file_path = os.path.join(root, file)
                if file_path not in existing_files:
                    scanned_count += 1
                    if scanned_count % 250 == 0:
                        print(f"Scanned {scanned_count} files.")
                    self.scan_file(file_path)

        self.graph = self.build_graph()
        self.auto_merge()
        self.searcher.search.cache_clear()

    @lru_cache(maxsize=None)
    def scan_file(self, file_path):
        try:
            album_artist_tag = True
            tags = self.extract_tags(file_path)
            if not tags:
                return
            
            song_name = tags['title'].strip()
            album_name = tags['album'].strip()
            artist_names = self.parse_artists(tags['artists'])
            album_artist_names = self.parse_artists(tags.get('album_artists', artist_names))
            track_number = tags['track_number']
            disc_number = tags['disc_number']
            year = tags['year']
            event_name = tags['event'][0].strip() if isinstance(tags['event'], list) else tags['event'].strip()

            if album_artist_names == artist_names:
                album_artist_tag = False
            
            # Handle album and artists
            album = self.find_or_create_album(album_name, album_artist_names, year, album_artist_tag)
            song_artists = self.find_or_create_artists(artist_names)
            
            # Handle event
            event = self.find_or_create_event(event_name, album)
            
            song_art_path = album.album_art_path if album.album_art_path else None

            song = Song(song_name, album, song_artists, file_path, track_number, disc_number, year, song_art_path, event)
            self.add_song(song)
            album.songs.append(song)
            album.update_year()
            album.update_event()
            if event:
                event.update_year()

            self.update_art_paths(album, song, song_artists)

        except Exception as e:
            print(f"Error processing file {file_path}: {e}")
            raise

    def find_or_create_album(self, album_name, album_artist_names, year, album_artist_tag):
        album = self.find_album_by_name_artist_year(album_name, tuple(album_artist_names), year, album_artist_tag)
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
        return album

    def find_or_create_artists(self, artist_names):
        artists = []
        for artist_name in artist_names:
            artist = self.find_artist_by_name(artist_name)
            if not artist:
                artist = Artist(artist_name)
                self.find_artist_by_name.cache_clear()
                self.add_artist(artist)
            artists.append(artist)
        return artists

    def find_or_create_event(self, event_name, album):
        if not event_name:
            return None
        event = self.find_event_by_name(event_name)
        if not event:
            print(f"Adding new event {event_name}.")
            event = Event(event_name)
            self.find_event_by_name.cache_clear()
            self.add_event(event)
        if album not in event.albums:
            event.albums.append(album)
        if not album.event or not album.event['uuid']:
            album.event = {'uuid': event.uuid, 'name': event.name}
        return event

    def update_art_paths(self, album, song, song_artists):
        if not album.album_art_path:
            album.album_art_path = song.song_art_path
            if not album.album_art_path:
                album.album_art_path = self.extract_embedded_art(song.file_path)
                if album.album_art_path:
                    for s in album.songs:
                        if not s.song_art_path:
                            s.song_art_path = album.album_art_path
                                
        for artist in album.album_artists:
            if not artist.artist_art_path:
                artist.artist_art_path = album.album_art_path

        for artist in song_artists:
            if not artist.artist_art_path:
                artist.artist_art_path = album.album_art_path
                if not artist.artist_art_path:
                    artist.artist_art_path = song.song_art_path


    def parse_artists(self, artists):
        delimiters = ('/', '／', '&', '＆', ' x ', ';', '；', ',', '，', '×', '　', '、')
        ignore = ['cool&create', 'Factory Noise&AG', 'Sing, R. Sing!']
        ignore_normalized = [utils.normalize_name(item) for item in ignore]
        parsed_artists = []

        for artist in artists:
            parts = []

            while artist:
                artist_normalized = utils.normalize_name(artist)
                for ignored in ignore_normalized:
                    ignored_index = artist_normalized.find(ignored)
                    if ignored_index != -1:
                        pre_ignored = artist[:ignored_index].strip()
                        ignored_part = artist[ignored_index:ignored_index + len(ignored)]
                        post_ignored = artist[ignored_index + len(ignored):].strip()

                        if pre_ignored:
                            parts.extend(utils.split_and_clean(pre_ignored, delimiters))

                        parts.append(ignored_part)
                        artist = post_ignored
                        break
                else:
                    parts.extend(utils.split_and_clean(artist, delimiters))
                    break

            parsed_artists.extend(parts)

        return parsed_artists
    
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
        
        # Update artist_art
        if artist1.artist_art_path == "" and artist2.artist_art_path != "":
            self.artists[uuid1].artist_art_path = self.artists[uuid2].artist_art_path
        
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

            if artist_updated and song.album['uuid'] in self.albums:
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