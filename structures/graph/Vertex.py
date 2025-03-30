from structures.geomtry.DiagonaDataclass import DiagonalDirection
from structures.geomtry.Vector import Vector
from structures.graph.TaggableElement import TaggableElement
from structures.graph.VertexNeighbourhood import VertexNeighbourhood
from structures.graph.VertexTag import VertexTag
from structures.graph.Tile import Tile


from typing import Final


class Vertex(TaggableElement):
    """
    A node in the layout graph. Every node corresponds uniquely to a corner of a Tile, and belongs exclusively to that Tile object.
    Stores:
     - Its location, via a Point
     - The owning Box.
     - A list of the adjacent Vertices (or Edges, as the case may be) in each cardinal direction (N,S,E,W)

    Vertex persist precisely with their owning tile, and should never be destroyed directly (only through the Tile). For this reason, its location is a separate object.
    """



    owner: Final[Tile]
    role: Final[DiagonalDirection]
    location: Vector[int] #not optional, so Windows should only be created via algorithms - canvass could possibly simulate free floating layers? or an extension of canvass.
    neighbours: Final[VertexNeighbourhood]

    def __init__(self, location: Vector[int], owner: Tile, role: DiagonalDirection):
        self.owner = owner
        self.role = role
        self.location = location
        self.neighbours = VertexNeighbourhood([],[],[],[])

    def generate_tag(self) -> VertexTag:
        return VertexTag(self.owner.generate_tag(), self.role)

    @property
    def debug_string(self) -> str:
        return f'{self.generate_tag()}@{self.location}'

    @property
    def is_sentinel(self):
        return self.owner.is_sentinel