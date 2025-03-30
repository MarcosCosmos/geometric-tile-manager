from dataclasses import dataclass
from functools import cache
from typing import NamedTuple

from geometry.axis import Axis
from geometry.graph.helpers import parse_tag
from geometry.graph.tag import Tag
from geometry.graph.vertex import Vertex, VertexTag


class Edge(NamedTuple):
    """
    Edges define connections between vertices which are axis aligned.

    Note: the a and b vertices must always be arranged in ascending order of their coordinate values.
        - Since one of their coordinate values is always the same, this means that a must have the lower value for the other coordinate.
        -- i.e. a must be either above or to the left of b, but not both.
        - the minimum distance for each edge is configurable, so the test is done outside of the class.

    Note: Edges can be 'fake' - they do not need to actually represent the sides of a tile and there may be a path of vertices along the edge.
    """

    a: Vertex
    b: Vertex

    def generate_tag(self) -> EdgeTag:
        return EdgeTag(self.a.generate_tag(), self.b.generate_tag())

    @property
    def debug_string(self) -> str:
        return f'{self.__class__.__name__}({self.a.debug_string} to {self.b.debug_string})'

    @property
    def is_side(self) -> bool:
        return self.a.owner is self.b.owner


    def __str__(self):
        return str(self.generate_tag())

    def __repr__(self):
        return str(self)

    @property
    @cache
    def axis(self) -> Axis:
        assert self.a.role.vertical == self.b.role.vertical or self.a.role.horizontal == self.b.role.horizontal
        return Axis.HORIZONTAL if self.a.role.vertical == self.b.role.vertical else Axis.VERTICAL

    def distance(self):
        """
        Since edges are axis-aligned, take the values along that axis
        :return:
        """
        return self.b.location[self.axis] - self.a.location[self.axis]


@dataclass
class EdgeTag(Tag[Edge]):
    format='{self.a.inner_str},{self.b.inner_str})'
    a: VertexTag
    b: VertexTag

    @staticmethod
    def parse(text: str) -> 'EdgeTag':
        return EdgeTag(*map(VertexTag.parse, parse_tag(EdgeTag.format, text)))
