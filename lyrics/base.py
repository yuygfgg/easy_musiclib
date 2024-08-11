import os
import importlib
from abc import ABC

class LyricsSource(ABC):

    @classmethod
    def load_sources(cls, directory):
        sources = []
        for filename in os.listdir(directory):
            if filename.endswith(".py") and filename != "base.py":
                module_name = f"lyrics.{filename[:-3]}"
                module = importlib.import_module(module_name)
                for attr in dir(module):
                    source_class = getattr(module, attr)
                    if (
                        isinstance(source_class, type)
                        and issubclass(source_class, cls)
                        and source_class is not cls
                    ):
                        sources.append(source_class())
        return sources

    @classmethod
    def get_lyrics(cls, title, artist, album, duration, directory="lyrics"):
        sources = cls.load_sources(directory)
        all_results = []

        for source in sources:
            results = source.get_aligned_lyrics(title, artist, album, duration)
            all_results.extend(results)

        all_results.sort(key=lambda x: x["similarity"], reverse=True)
        return all_results[:9]
