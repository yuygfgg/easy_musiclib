from datetime import datetime
import uuid
import os
from musiclib import utils

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


    def find_art_path(self, file_path):
        folder_path = os.path.dirname(file_path)
        possible_files = utils.generate_possible_art_files()
        
        for art_file in possible_files:
            art_path = os.path.join(folder_path, art_file)
            if os.path.isfile(art_path):
                return art_path
        
        return utils.extract_embedded_art(file_path)

    def like(self):
        self.is_liked = True
        self.liked_time = datetime.now()
        print(f"Song {self.name} liked.")

    def unlike(self):
        self.is_liked = False
        print(f"Song {self.name} unliked.")