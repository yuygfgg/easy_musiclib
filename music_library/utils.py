from functools import lru_cache
import itertools
from multiprocessing import Pool, cpu_count
import os
import re
import shutil
import subprocess
import tempfile
from rapidfuzz import fuzz
import opencc


cc = opencc.OpenCC("t2s")


def extract_embedded_art(file_path):
    try:
        with tempfile.TemporaryDirectory() as temp_dir:
            art_path = os.path.join(temp_dir, "folder.jpg")

            ffmpeg_command = [
                "ffmpeg",
                "-i",
                file_path,
                "-an",
                "-vcodec",
                "mjpeg",
                "-update",
                "1",
                "-y",
                art_path,
            ]

            subprocess.run(ffmpeg_command, check=True, shell=False)

            if os.path.isfile(art_path):
                final_art_path = os.path.join(os.path.dirname(file_path), "folder.jpg")
                shutil.copyfile(art_path, final_art_path)
                return final_art_path
    except subprocess.CalledProcessError as e:
        print(f"Error extracting embedded art from {file_path}: {e}")
    except Exception as e:
        print(f"Unexpected error: {e}")
    return ""


@lru_cache(maxsize=None)
def generate_possible_art_files():
    art_files = [
        "folder.jpg",
        "cover.jpg",
        "folder.png",
        "cover.png",
        "folder.tif",
        "cover.tif",
        "folder.tiff",
        "cover.tiff",
        "folder.jpeg",
        "cover.jpeg",
    ]

    possible_files = set()
    for art_file in art_files:
        # 生成所有可能的大小写组合
        base_name, ext = os.path.splitext(art_file)
        for combo in itertools.product(
            *((char.lower(), char.upper()) for char in base_name)
        ):
            possible_files.add("".join(combo) + ext.lower())

    return possible_files


@lru_cache(maxsize=None)
def extract_year(date_string):
    if date_string:
        match = re.search(r"\b(\d{4})\b", date_string)
        if match:
            return int(match.group(1))
    return None


@lru_cache(maxsize=None)
def normalize_name(name, for_search=False):
    normalized_name = cc.convert(name.strip().lower())
    normalized_name = normalized_name.translate(
        str.maketrans(
            "ァィゥェォャュョッーアイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワヲンヴヵヶ",
            "ぁぃぅぇぉゃゅょっーあいうえおかきくけこさしすせそたちつてとなにぬねのはひふへほまみむめもやゆよらりるれろわをんゔゕゖ",
        )
    )
    normalized_name = (
        re.sub(r"\s+", "", normalized_name) if for_search else normalized_name
    )
    return normalized_name


@lru_cache(maxsize=None)
def combined_score(attribute, query):
    normalized_query = normalize_name(query, True)
    normalized_attribute = normalize_name(attribute, True)

    def get_match_score(normalized_query, normalized_attribute):
        return fuzz.ratio(normalized_query, normalized_attribute)

    match_score = get_match_score(normalized_query, normalized_attribute)
    length_score = max(0, (1 - abs(len(attribute) - len(query)) / len(query)) * 100)
    return match_score * 0.7 + length_score * 0.3


def calculate_scores(items, attribute_getter, query):
    with Pool(cpu_count()) as pool:
        scores = pool.starmap(
            combined_score, [(attribute_getter(item), query) for item in items]
        )
    return scores


def get_song_name(song):
    return song.name


def get_album_name(album):
    return album.name


def get_artist_name(artist):
    return artist.name


@lru_cache(maxsize=None)
def split_and_clean(text, delimiters):
    temp_artists = [text]
    for delimiter in delimiters:
        new_temp_artists = []
        for temp_artist in temp_artists:
            new_temp_artists.extend(
                [
                    sub_artist.strip()
                    for sub_artist in temp_artist.split(delimiter)
                    if sub_artist.strip()
                ]
            )
        temp_artists = new_temp_artists
    return temp_artists


def parse_artists(artists):
    delimiters = (
        "/",
        "／",
        "&",
        "＆",
        " x ",
        ";",
        "；",
        ",",
        "，",
        "×",
        "　",
        "、",
    )
    ignore = ["cool&create", "Factory Noise&AG", "Sing, R. Sing!"]
    ignore_normalized = [normalize_name(item) for item in ignore]
    parsed_artists = []

    for artist in artists:
        parts = []

        while artist:
            artist_normalized = normalize_name(artist)
            for ignored in ignore_normalized:
                ignored_index = artist_normalized.find(ignored)
                if ignored_index != -1:
                    pre_ignored = artist[:ignored_index].strip()
                    ignored_part = artist[ignored_index : ignored_index + len(ignored)]
                    post_ignored = artist[ignored_index + len(ignored) :].strip()

                    if pre_ignored:
                        parts.extend(split_and_clean(pre_ignored, delimiters))

                    parts.append(ignored_part)
                    artist = post_ignored
                    break
            else:
                parts.extend(split_and_clean(artist, delimiters))
                break

        parsed_artists.extend(parts)

    return parsed_artists

def is_year(n):
    return bool(re.match(r'^-?\d{4}$', str(n)))