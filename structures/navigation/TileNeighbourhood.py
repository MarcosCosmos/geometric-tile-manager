from __future__ import annotations

from typing import Final

from structures.graph import Tile


class TileNeighbourhood:
    owner: Final[Tile]
    def __init__(self, owner: Tile):
        self.owner = owner
