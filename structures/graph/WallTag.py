from structures.graph.Tag import Tag
from structures.graph.TileTag import TileTag
from structures.graph.Wall import Wall


class WallTag(TileTag, Tag[Wall]):
    ...