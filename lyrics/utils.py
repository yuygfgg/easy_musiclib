from difflib import SequenceMatcher


def get_similarity(a, b):
        return SequenceMatcher(None, a, b).ratio()