import requests
import tkinter as tk
from tkinter import messagebox, filedialog, simpledialog
import json
import subprocess
import os

class MusicLibraryClient:
    def __init__(self, base_url="http://127.0.0.1:5000"):
        self.base_url = base_url
        self.headers = {"Content-Type": "application/json"}

    def add_song(self, name, album_uuid, artist_uuids, file_path, track_number=1, disc_number=1):
        params = {
            "name": name,
            "album_uuid": album_uuid,
            "artist_uuids": artist_uuids,
            "file_path": file_path,
            "track_number": track_number,
            "disc_number": disc_number
        }
        response = requests.get(f"{self.base_url}/add_song", params=params, headers=self.headers)
        return response.json()

    def add_album(self, name):
        params = {"name": name}
        response = requests.get(f"{self.base_url}/add_album", params=params, headers=self.headers)
        return response.json()

    def add_artist(self, name):
        params = {"name": name}
        response = requests.get(f"{self.base_url}/add_artist", params=params, headers=self.headers)
        return response.json()

    def search_song(self, name):
        response = requests.get(f"{self.base_url}/search_song/{name}", headers=self.headers)
        return response.json()

    def search_album(self, name):
        response = requests.get(f"{self.base_url}/search_album/{name}", headers=self.headers)
        return response.json()

    def search_artist(self, name):
        response = requests.get(f"{self.base_url}/search_artist/{name}", headers=self.headers)
        return response.json()

    def like_song(self, uuid):
        response = requests.get(f"{self.base_url}/like_song/{uuid}", headers=self.headers)
        return response.json()

    def unlike_song(self, uuid):
        response = requests.get(f"{self.base_url}/unlike_song/{uuid}", headers=self.headers)
        return response.json()

    def like_album(self, uuid):
        response = requests.get(f"{self.base_url}/like_album/{uuid}", headers=self.headers)
        return response.json()

    def unlike_album(self, uuid):
        response = requests.get(f"{self.base_url}/unlike_album/{uuid}", headers=self.headers)
        return response.json()

    def like_artist(self, uuid):
        response = requests.get(f"{self.base_url}/like_artist/{uuid}", headers=self.headers)
        return response.json()

    def unlike_artist(self, uuid):
        response = requests.get(f"{self.base_url}/unlike_artist/{uuid}", headers=self.headers)
        return response.json()

    def show_library(self):
        response = requests.get(f"{self.base_url}/show_library", headers=self.headers)
        return response.json()

    def show_liked_songs(self):
        response = requests.get(f"{self.base_url}/show_liked_songs", headers=self.headers)
        return response.json()

    def show_liked_artists(self):
        response = requests.get(f"{self.base_url}/show_liked_artists", headers=self.headers)
        return response.json()

    def show_liked_albums(self):
        response = requests.get(f"{self.base_url}/show_liked_albums", headers=self.headers)
        return response.json()

    def scan(self, directory):
        params = {"directory": directory}
        response = requests.get(f"{self.base_url}/scan", params=params, headers=self.headers)
        return response.json()

    def show_song(self, uuid):
        response = requests.get(f"{self.base_url}/show_song/{uuid}", headers=self.headers)
        return response.json()

    def show_album(self, uuid):
        response = requests.get(f"{self.base_url}/show_album/{uuid}", headers=self.headers)
        return response.json()

    def show_artist(self, uuid):
        response = requests.get(f"{self.base_url}/show_artist/{uuid}", headers=self.headers)
        return response.json()

    def search(self, query):
        response = requests.get(f"{self.base_url}/search/{query}", headers=self.headers)
        return response.json()

    def get_file(self, file_path):
        params = {"file_path": file_path}
        response = requests.get(f"{self.base_url}/getfile", params=params, headers=self.headers)
        return response.content


class MusicLibraryGUI:
    def __init__(self, root):
        self.client = MusicLibraryClient()
        self.root = root
        self.root.title("Music Library")

        self.frame = tk.Frame(root)
        self.frame.pack(padx=10, pady=10)

        self.search_label = tk.Label(self.frame, text="Search")
        self.search_label.grid(row=0, column=0, padx=5, pady=5)
        self.search_entry = tk.Entry(self.frame)
        self.search_entry.grid(row=0, column=1, padx=5, pady=5)
        self.search_button = tk.Button(self.frame, text="Search", command=self.search)
        self.search_button.grid(row=0, column=2, padx=5, pady=5)

        self.result_text = tk.Text(self.frame, height=20, width=80)
        self.result_text.grid(row=1, column=0, columnspan=3, padx=5, pady=5)

        self.play_button = tk.Button(self.frame, text="Play Song", command=self.play_song)
        self.play_button.grid(row=2, column=0, padx=5, pady=5)
        self.like_button = tk.Button(self.frame, text="Like Song", command=self.like_song)
        self.like_button.grid(row=2, column=1, padx=5, pady=5)
        self.unlike_button = tk.Button(self.frame, text="Unlike Song", command=self.unlike_song)
        self.unlike_button.grid(row=2, column=2, padx=5, pady=5)

    def search(self):
        query = self.search_entry.get()
        if not query:
            messagebox.showerror("Error", "Please enter a search query")
            return

        result = self.client.search(query)
        self.result_text.delete(1.0, tk.END)
        self.result_text.insert(tk.END, json.dumps(result, indent=4))

    def play_song(self):
        uuid = simpledialog.askstring("Input", "Enter UUID of the song to play:")
        if not uuid:
            return

        song_info = self.client.show_song(uuid)
        self.result_text.delete(1.0, tk.END)
        self.result_text.insert(tk.END, json.dumps(song_info, indent=4))

        file_path = song_info.get("file_path")
        if not file_path:
            messagebox.showerror("Error", "File path not found for the song")
            return

        file_content = self.client.get_file(file_path)
        temp_file_path = "temp_song.mp3"
        temp_art_path = "temp_art.jpg"

        with open(temp_file_path, "wb") as f:
            f.write(file_content)

        art_path = song_info.get("song_art_path")
        if art_path:
            art_content = self.client.get_file(art_path)
            with open(temp_art_path, "wb") as f:
                f.write(art_content)
            subprocess.run(["mpv", temp_file_path, "--image-display-duration=inf", f"--external-file={temp_art_path}"])
            os.remove(temp_art_path)
        else:
            subprocess.run(["mpv", temp_file_path])

        os.remove(temp_file_path)

    def like_song(self):
        uuid = simpledialog.askstring("Input", "Enter UUID of the song to like:")
        if not uuid:
            return

        result = self.client.like_song(uuid)
        messagebox.showinfo("Info", json.dumps(result, indent=4))

    def unlike_song(self):
        uuid = simpledialog.askstring("Input", "Enter UUID of the song to unlike:")
        if not uuid:
            return

        result = self.client.unlike_song(uuid)
        messagebox.showinfo("Info", json.dumps(result, indent=4))


if __name__ == "__main__":
    root = tk.Tk()
    app = MusicLibraryGUI(root)
    root.mainloop()