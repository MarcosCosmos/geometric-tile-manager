from structures.geomtry import DiagonalDataclass
from structures.graph import Box, Tag, VertexTag
from structures.graph.helpers import parse_tag
from utility import enum_dataclass


@enum_dataclass
class BoxTag(DiagonalDataclass[VertexTag], Tag[Box]):
    format='{self.north_west.inner_str},{self.north_east.inner_str},{self.south_east.inner_str},{self.south_west.inner_str}'
    #
    # def __iter__(self) -> Iterator[VertexTag]:
    #     return iter([self.north_west, self.north_east, self.south_east, self.south_west])
    #
    # def __getitem__(self, key: DiagonalDirection) -> VertexTag:
    #     return getattr(self, key.snake_case_name)
    #
    # def __setitem__(self, key: DiagonalDirection, value: VertexTag):
    #     setattr(self, key.snake_case_name, value)

    @staticmethod
    def parse(text: str) -> 'BoxTag':
        return BoxTag(*map(VertexTag.parse, parse_tag(BoxTag.format, text)))