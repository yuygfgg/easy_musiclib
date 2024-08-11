import uuid
from datetime import datetime


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
        self.event = {"uuid": None, "name": None}

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
        if not self.event["uuid"] and self.songs:
            for song in self.songs:
                if song.event["uuid"]:
                    self.event = song.event
                    break
