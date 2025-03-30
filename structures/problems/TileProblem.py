from structures.graph import Tile
from structures.problems import BoxProblem


class TileProblem(BoxProblem):
    box: Tile

    def __init__(self, box: Tile):
        super().__init__(box)

    @property
    def tile(self) -> Tile:
        return self.box