from typing import NamedTuple


class TileId(NamedTuple):
    """
    Wrapper for internally generated tile ids to prevent name conflicts with e.g. actual OS window ids, which may be commonly used as names in practice.
    """
    number: int
    def __str__(self):
        return f'<{self.number}>'