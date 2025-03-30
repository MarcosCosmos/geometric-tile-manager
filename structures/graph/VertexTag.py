from structures.geomtry import DiagonalDirection
from structures.graph import Tag, Vertex, TileTag

from dataclasses import dataclass

from structures.graph.helpers import parse_tag


@dataclass
class VertexTag(Tag[Vertex]):
    format='{self.owner.inner_str}.{self.role.snake_case_name}'
    owner: TileTag
    role: DiagonalDirection

    @staticmethod
    def parse(text: str) -> 'VertexTag':
        owner, role = parse_tag(VertexTag.format, text)
        return VertexTag(TileTag.parse(owner), DiagonalDirection.__getitem__[role.upper()])