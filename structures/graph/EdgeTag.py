from structures.graph import Edge, Tag, VertexTag
from structures.graph.helpers import parse_tag


from dataclasses import dataclass


@dataclass
class EdgeTag(Tag[Edge]):
    format='{self.a.inner_str},{self.b.inner_str})'
    a: VertexTag
    b: VertexTag

    @staticmethod
    def parse(text: str) -> 'EdgeTag':
        return EdgeTag(*map(VertexTag.parse, parse_tag(EdgeTag.format, text)))