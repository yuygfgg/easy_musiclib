from functools import lru_cache
from . import utils


class MusicLibrarySearch:
    @lru_cache(maxsize=None)
    def search(self, query):
        normalized_query = utils.normalize_name(query, True)
        print(normalized_query)

        song_scores = utils.calculate_scores(
            self.songs.values(), utils.get_song_name, normalized_query
        )
        album_scores = utils.calculate_scores(
            self.albums.values(), utils.get_album_name, normalized_query
        )
        artist_scores = utils.calculate_scores(
            self.artists.values(), utils.get_artist_name, normalized_query
        )

        matched_songs = [
            (song, score)
            for song, score in zip(self.songs.values(), song_scores)
            if score > 50
        ]
        matched_albums = [
            (album, score)
            for album, score in zip(self.albums.values(), album_scores)
            if score > 50
        ]
        matched_artists = [
            (artist, score)
            for artist, score in zip(self.artists.values(), artist_scores)
            if score > 50
        ]

        # Sort by score in descending order
        matched_songs.sort(key=lambda x: x[1], reverse=True)
        matched_albums.sort(key=lambda x: x[1], reverse=True)
        matched_artists.sort(key=lambda x: x[1], reverse=True)

        # Limit the results
        limited_songs = [song for song, score in matched_songs[:200]]
        limited_albums = [album for album, score in matched_albums[:50]]
        limited_artists = [artist for artist, score in matched_artists[:50]]

        return {
            "songs": limited_songs,
            "albums": limited_albums,
            "artists": limited_artists,
        }
