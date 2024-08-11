import uuid
from datetime import datetime


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
