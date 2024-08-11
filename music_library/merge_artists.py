import csv
import os


class MusicLibraryMergeArtist:
    def auto_merge(self):
        file_path = os.path.join(os.getcwd(), "artist_alias.csv")

        if not os.path.isfile(file_path):
            print("File artist_alias.csv not found in the current directory.")
            return

        with open(file_path, newline="", encoding="utf-8-sig") as csvfile:
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
                        print(
                            f"Merging alias artist '{alias_name}' into primary artist '{primary_artist_name}'"
                        )
                        self.merge_artist_by_name(primary_artist_name, alias_name)
                        print(
                            f"Successfully merged '{alias_name}' into '{primary_artist_name}'"
                        )

        for event in self.events.values():
            for album in event.albums:
                if album.year is None:
                    print(
                        f"Setting year for album {album.name} and its songs to {event.year}"
                    )
                    album.year = event.year
                    for song in album.songs:
                        song.year = event.year

        for album in self.albums.values():
            if album.album_art_path:
                for song in album.songs:
                    if not song.song_art_path:
                        print(
                            f"Setting song art for song {song.name} to album art {album.album_art_path}"
                        )
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
                if artist["uuid"] == uuid2:
                    artist["uuid"] = uuid1
                    artist["name"] = artist1.name
                    artist_updated = True

            if artist_updated and song.album["uuid"] in self.albums:
                album = self.albums[song.album["uuid"]]
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
                    self.graph[uuid1][neighbor]["strength"] += data["strength"]
                    self.graph[uuid1][neighbor]["details"].update(data["details"])
                if neighbor in self.graph:
                    if uuid2 in self.graph[neighbor]:
                        del self.graph[neighbor][uuid2]
                    if uuid1 not in self.graph[neighbor]:
                        self.graph[neighbor][uuid1] = data
                    else:
                        self.graph[neighbor][uuid1]["strength"] += data["strength"]
                        self.graph[neighbor][uuid1]["details"].update(data["details"])
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