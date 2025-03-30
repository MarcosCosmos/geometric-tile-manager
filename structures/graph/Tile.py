from structures.geomtry import CardinalDirection, DiagonalDirection, Vector
from structures.graph import Box, TileId, TileTag, Vertex, Canvas


from abc import abstractmethod
from typing import Final, Optional


class Tile(Box):
    """
    Concrete Boxes that create and strictly own the Vertices that define their corners.

    In addition to the corresponding vertices, Tile have unique identifiers.
    """

    id:  Final[TileId]
    _name: Optional[str]

    def __init__(self, id: TileId, north_west: Vector, north_east: Vector, south_east: Vector, south_west: Vector, *, name=None):
        super().__init__(*(Vertex(location, self, direction) for (location, direction) in zip((north_west, north_east, south_east, south_west), DiagonalDirection)))

        self.id = id
        self._name = name

        # connect the vertices along each side to one another
        for each_direction in CardinalDirection:
            perpendicular_directions = each_direction.axis.perpendicular.directions
            each_side = self.sides[each_direction]
            each_side[0].neighbours[perpendicular_directions[-1]] = [each_side[-1]]
            each_side[-1].neighbours[perpendicular_directions[0]] = [each_side[0]]

    @abstractmethod
    def generate_tag(self) -> TileTag:
        ...

    @property
    def debug_string(self) -> str:
        return f"{self.generate_tag()}@({','.join(each.debug_string for each in self.corners)}"

    @property
    def name(self):
        return self._name
    @property
    def is_sentinel(self):
        return isinstance(self, Canvas)