class MusicLibraryDisplay:
    def __init__(self, music_library):
        self.music_library = music_library

    def show_songinfo(self, song):
        print("------------start song information------------")
        print(f"Song: {song.name}")
        print(f"UUID: {song.uuid}")
        print(f"YEAR: {song.year}")
        print(f"Album: {song.album['name']} (UUID: {song.album['uuid']})")
        artist_info = ", ".join(
            [f"{artist['name']} (UUID: {artist['uuid']})" for artist in song.artists]
        )
        print(f"Artists: {artist_info}")
        print(f"File Path: {song.file_path}")
        print(f"Track Number: {song.track_number}")
        print(f"Disc Number: {song.disc_number}")
        print(f"Song Art Path: {song.song_art_path}")
        print(f"Isliked: {song.is_liked}")
        print(f"Year: {song.year}")
        print(f"Event: {song.event}")
        print("------------end song information------------")

    def show_albuminfo(self, album):
        print("------------start album information------------")
        print(f"Album: {album.name}")
        print(f"UUID: {album.uuid}")
        print(f"YEAR: {album.year}")
        for artist in album.album_artists:
            print(f"Album artist: {artist.name} ({artist.uuid})")
        print("Songs:")
        for song in album.songs:
            print(
                f"  - {song.name} (UUID: {song.uuid}, Track: {song.track_number}, Disc: {song.disc_number})"
            )
        print(f"Album Art Path: {album.album_art_path}")
        print(f"Isliked: {album.is_liked}")
        print(f"Year: {album.year}")
        print(f"Event: {album.event}")
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
        for album in self.music_library.albums.values():
            if artist in album.album_artists:
                print(f"  - {album.name} (UUID: {album.uuid})")
        print("Songs:")
        for song in self.music_library.songs.values():
            if artist.uuid in [a["uuid"] for a in song.artists]:
                print(
                    f"  - {song.name} (UUID: {song.uuid}, Track: {song.track_number}, Disc: {song.disc_number})"
                )
        print(f"Artist Art Path: {artist.artist_art_path}")
        print(f"Isliked: {artist.is_liked}")

    def show_library(self):
        print("Artists:")
        for artist in self.music_library.artists.values():
            self.show_artistinfo(artist)
        print("\nAlbums:")
        for album in self.music_library.albums.values():
            self.show_albuminfo(album)
        print("\nSongs:")
        for song in self.music_library.songs.values():
            self.show_songinfo(song)

    def show_liked_songs(self):
        liked_songs = [
            song for song in self.music_library.songs.values() if song.is_liked
        ]
        for song in liked_songs:
            print(f"{song.name} (UUID: {song.uuid}) Disc_number: {song.disc_number}")

    def show_liked_artists(self):
        liked_artists = [
            artist for artist in self.music_library.artists.values() if artist.is_liked
        ]
        for artist in liked_artists:
            print(f"{artist.name} (UUID: {artist.uuid})")

    def show_liked_albums(self):
        liked_albums = [
            album for album in self.music_library.albums.values() if album.is_liked
        ]
        for album in liked_albums:
            print(f"{album.name} (UUID: {album.uuid})")
