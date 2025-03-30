import parse as ps

from structures.graph.Tag import Tag
from structures.graph.Tile import Tile
from structures.graph.TileId import TileId


from dataclasses import dataclass


@dataclass
class TileTag(Tag[Tile]):
    format='{self.id}'
    id: TileId | str

    @staticmethod
    def parse(id: str) -> 'TileTag':
        match = ps.parse("<{:d}>", id)
        if match is None:
            return TileTag(id)
        else:
            return TileTag(TileId(*match.fixed))