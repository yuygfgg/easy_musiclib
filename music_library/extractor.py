import importlib
import os

from .tag_extracter import TagExtractor


class MusicLibraryTagExtractor:
    def load_extractors(self):
        extractors = {}
        for file in os.listdir("music_library/tag_extracter"):
            if file.endswith("_extractor.py"):
                module_name = file[:-3]
                module = importlib.import_module(
                    f"music_library.tag_extracter.{module_name}"
                )
                for attr in dir(module):
                    cls = getattr(module, attr)
                    if (
                        isinstance(cls, type)
                        and issubclass(cls, TagExtractor)
                        and cls is not TagExtractor
                    ):
                        # Extract the extension from the module name
                        extension = module_name.split("_")[0]
                        extractors[extension] = cls()
        return extractors

    def extract_tags(self, file_path):
        extension = os.path.splitext(file_path)[1][1:].lower()
        extractor = self.extractors.get(extension)
        if extractor:
            return extractor.extract(file_path)
        return None
