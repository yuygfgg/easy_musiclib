import uuid
from datetime import datetime

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