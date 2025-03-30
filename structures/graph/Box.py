from structures.geomtry import CardinalDirection
from structures.geomtry import DiagonalDataclass
from structures.graph import Edge
from structures.graph import TaggableElement
from structures.graph import Vertex
from structures.graph import BoxTag


from typing import Final, overload

from utility import enum_dataclass


class Box(TaggableElement):
    """
    Abstract base class for regions of the tiling space, such as Tiles, Rulers and SegmentedBoxes.
    Although only subclasses are practically useful for the most part, boxes can be tagged and instantiated in their own right for inspection and decision-making purposes.

    Boxes are allowed to cover multiple tiles and partial tiles in principle, although subclasses tend to have more specific rules.
    """
    @enum_dataclass
    class Corners(DiagonalDataclass[Vertex]):
        ...

    class Sides:
        """
        Syntactic sugar over generating an edge corresponding to a side of the box.
        """
        _owner: Final['Box']
        def __init__(self, owner: 'Box'):
            self._owner = owner

        def __getitem__(self, key: CardinalDirection) -> Edge:
            return Edge(*map(self._owner.corners.__getitem__, key.diagonals))

        def __iter__(self):
            return iter(map(self.__getitem__, CardinalDirection))

    corners: Final[Corners]

    @overload
    def __init__(self, north_west: Vertex, north_east: Vertex, south_east: Vertex, south_west: Vertex):
        ...

    def __init__(self, *args, **kwargs):
        self.corners: Box.Corners = Box.Corners(*args, **kwargs)

    @property
    def sides(self):
        return Box.Sides(self)

    def generate_tag(self) -> BoxTag:
        return BoxTag(*map(Vertex.generate_tag, self.corners))

    @property
    def debug_string(self) -> str:
        return f"Box({','.join(each.debug_string for each in self.corners)}"