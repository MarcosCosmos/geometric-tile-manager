from dataclasses import dataclass
from typing import Final, TYPE_CHECKING

from geometry.direction.diagonal import DiagonalDirection
from geometry.vector import Vector
from geometry.graph.helpers import parse_tag
from geometry.graph.tag import TaggableElement, Tag


if TYPE_CHECKING:
    from geometry.graph.tile import Tile, TileTag
    from geometry.graph.neighbourhood import VertexNeighbourhood



class Vertex(TaggableElement):
    """
    A node in the layout graph. Every node corresponds uniquely to a corner of a Tile, and belongs exclusively to that Tile object.
    Stores:
     - Its location, via a Point
     - The owning Box.
     - A list of the adjacent Vertices (or Edges, as the case may be) in each cardinal direction (N,S,E,W)

    Vertex persist precisely with their owning tile, and should never be destroyed directly (only through the Tile). For this reason, its location is a separate object.
    """



    owner: Final['Tile']
    role: Final[DiagonalDirection]
    location: Vector[int] #not optional, so Windows should only be created via algorithms - canvass could possibly simulate free floating layers? or an extension of canvass.
    neighbours: Final['VertexNeighbourhood']

    def __init__(self, location: Vector[int], owner: 'Tile', role: DiagonalDirection):
        from geometry.graph.neighbourhood import VertexNeighbourhood
        self.owner = owner
        self.role = role
        self.location = location
        self.neighbours = VertexNeighbourhood([],[],[],[])

    def generate_tag(self) -> 'VertexTag':
        return VertexTag(self.owner.generate_tag(), self.role)

    @property
    def debug_string(self) -> str:
        return f'{self.generate_tag()}@{self.location}'

    @property
    def is_sentinel(self):
        return self.owner.is_sentinel


@dataclass
class VertexTag(Tag[Vertex]):
    format='{self.owner.inner_str}.{self.role.snake_case_name}'
    owner: 'TileTag'
    role: DiagonalDirection

    @staticmethod
    def parse(text: str) -> 'VertexTag':
        from geometry.graph.tile import TileTag
        owner, role = parse_tag(VertexTag.format, text)
        return VertexTag(TileTag.parse(owner), DiagonalDirection[role.upper()])
