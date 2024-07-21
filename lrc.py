from flask import Flask, request, Response, jsonify
import requests
import re
import os
from difflib import SequenceMatcher

LRC_DIRECTORY = os.environ.get('LRC_DIRECTORY', '/Users/a1/other')

def search_song(keyword):
    api_url = f"https://music.163.com/api/search/get?s={keyword}&type=1&limit=50"
    response = requests.get(api_url)
    if response.status_code != 200:
        return None
    return response.json().get('result')

def download_lyrics(song_id):
    url = "https://music.163.com/api/song/lyric"
    params = {"tv": "-1", "lv": "-1", "kv": "-1", "id": song_id}
    response = requests.get(url, params=params)
    if response.status_code != 200:
        return None, None
    data = response.json()
    lrc = data.get('lrc', {}).get('lyric', '')
    tlyric = data.get('tlyric', {}).get('lyric', '')
    return lrc, tlyric

def attempt_to_download_lyrics_from_songs(songs):
    print("Starting to attempt to download lyrics for songs list:")
    print(songs)
    for index, song in enumerate(songs):
        print(f"Trying song {index + 1}/{len(songs)} with id {song['id']}")
        try:
            lyrics_content, trans_lyrics_content = download_lyrics(song['id'])
            if lyrics_content:
                lines = lyrics_content.count('\n') + 1
                if lines > 5:
                    print(f"Lyrics with more than 5 lines found for song with id {song['id']}")
                    print(lyrics_content)
                    return lyrics_content, trans_lyrics_content 
                else:
                    print(f"Lyrics found for song with id {song['id']} but less than 6 lines")
            else:
                print(f"No lyrics found for song with id {song['id']}")
        except Exception as e:
            print(f"Error downloading lyrics for song with id {song['id']}: {e}")
    
    print("No suitable lyrics found for any songs in the list.")
    return None, None

def parse_lyrics(lyrics):
    lyrics_dict = {}
    unformatted_lines = []
    pattern = re.compile(r'\[(\d{2}):(\d{2})([.:]\d{2,3})?\](.*)')
    for line in lyrics.split('\n'):
        match = pattern.match(line)
        if match:
            minute, second, millisecond, lyric = match.groups()
            millisecond = millisecond if millisecond else '.000'
            millisecond = millisecond.replace(':', '.')
            time_stamp = f"[{minute}:{second}{millisecond}]"
            lyrics_dict[time_stamp] = lyric
        else:
            unformatted_lines.append(line)
    return lyrics_dict, unformatted_lines

def merge_lyrics(lrc_dict, tlyric_dict, unformatted_lines):
    merged_lyrics = unformatted_lines
    all_time_stamps = sorted(set(lrc_dict.keys()).union(tlyric_dict.keys()))
    for time_stamp in all_time_stamps:
        original_line = lrc_dict.get(time_stamp, '')
        translated_line = tlyric_dict.get(time_stamp, '')
        merged_lyrics.append(f"{time_stamp}{original_line}")
        if translated_line:
            merged_lyrics.append(f"{time_stamp}{translated_line}")
    return '\n'.join(merged_lyrics)

def check_local_lyrics(title):
    for file in os.listdir(LRC_DIRECTORY):
        if file.startswith(title) and file.endswith('.lrc'):
            with open(os.path.join(LRC_DIRECTORY, file), 'r', encoding='utf-8') as f:
                return f.read()
    return None

def get_similarity(a, b):
    return SequenceMatcher(None, a, b).ratio()

def get_aligned_lyrics(title, artist, album, duration):
    search_keywords = [
        f"{artist} - {album} - {title}",
        f"{album} - {title}",
        f"{artist} - {title}",
        f"{title}"
    ]

    results = []

    for keyword in search_keywords:
        search_result = search_song(keyword)
        if not search_result:
            continue

        songs = search_result.get('songs', [])
        songs = [song for song in songs if abs(song['duration'] / 1000 - duration) <= 10]

        for song in songs[:3]:
            lyrics_content, trans_lyrics_content = download_lyrics(song['id'])

            if lyrics_content:
                lrc_dict, unformatted_lines = parse_lyrics(lyrics_content)
                if len(lrc_dict) >= 5:
                    tlyric_dict, _ = parse_lyrics(trans_lyrics_content if trans_lyrics_content else '')
                    merged = merge_lyrics(lrc_dict, tlyric_dict, unformatted_lines)
                    similarity = (
                        get_similarity(title, song['name']) +
                        get_similarity(artist, ', '.join([artist['name'] for artist in song['artists']])) +
                        get_similarity(album, song.get('album', {}).get('name', ''))
                    ) / 3
                    results.append({
                        "id": str(song['id']),
                        "title": song['name'],
                        "artist": ', '.join([artist['name'] for artist in song['artists']]),
                        "lyrics": merged,
                        "similarity": similarity
                    })
            
    results.sort(key=lambda x: x['similarity'], reverse=True)
    return results