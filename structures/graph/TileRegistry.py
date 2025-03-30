import itertools
import warnings
from functools import singledispatchmethod
from typing import Final, Iterator, Mapping, MutableMapping, Sequence, Type, TypeVar, overload

from structures.geomtry import DiagonalDirection, Vector
from structures.graph import Tile, TileId, Window, Canvas, TileTag, BoxTag, Box, WindowTag, SpaceTag, VertexTag, Vertex, \
    EdgeTag, Edge, Tag

T = TypeVar('T')
TileT = TypeVar('TileT', bound=Tile)
class TileRegistry:
    """
    TileRegistries are Tile factories and provide storage/lookup functionality.

    It also acts as a factory for all tiles, to ensure that it maintains access

    Rules for uniquely identifying and describing graph objects:
    - Canvas ids are distinct from Window Ids, so the full descriptor for a tile must include its type.
    - Vertices are described by their

    This registry is a factory, but it does not manage tile linkage or geometry, it only provides lookup and factory functionality.
    """
    _by_name: Final[MutableMapping[str, Tile]]
    _by_type: Final[Mapping[Type[TileT], MutableMapping[TileId, TileT]]]
    _tile_id_iter: Iterator[int]

    @property
    def by_type(self) -> Mapping[Type[TileT], Mapping[TileId, TileT]]:
        return self._by_type

    def __init__(self):
        self._tile_id_iter = itertools.count()
        self._by_type = {
            Window: {},
            Canvas: {}
        }
        self._by_name = {}

    @overload
    def create_tile(self, tile_class: Type[TileT], corners: dict[DiagonalDirection, Vector], *, name=None) -> TileT:
        ...

    @overload
    def create_tile(self, tile_class: Type[TileT], position: Vector, size: Vector, *, name=None) -> TileT:
        ...

    def create_tile(self, tile_class: Type[TileT], *args, **kwargs) -> TileT:
        corners: Sequence[Vector]
        if len(args) == 1:
            corners = tuple(args[0][each] for each in DiagonalDirection)
        if len(args) == 2:
            position: Vector
            size: Vector
            (position, size) = args
            corners = (position, position + Vector(size.horizontal, 0), position + size, position + Vector(0, size.vertical))
        elif len(args) == 4:
            corners = args
        name=kwargs['name'] if 'name' in kwargs else None
        result = tile_class(TileId(next(self._tile_id_iter)), *corners, name=name)
        self._by_type[tile_class][result.id] = result
        if name is not None:
            self.name_tile(result, name)
        return result

    def _erase_tile(self, key: TileTag):
        result = self[key]
        if result._name is not None:
            self.unname_tile(result)
        del self._by_type[result.__class__][result.id]

    def name_tile(self, target: Tile, name: str):
        """
        Register a name for a tile. The registered name will appear in all internally created tags for the tile, and can be used instead of the numerical id to create new tags for it.
        :param target:
        :param name: cannot be convertable to a TileId (that is, if it is a string, int(name) must fail.
        :return:
        """
        if name in self._by_name:
            warnings.warn("Naming a tile with a name already in use (the previous owner of the name will be unnammed)")
            self.unname_tile(self._by_name[name])
        target._name = name
        self._by_name[name] = target
        # valid = True
        # try:
        #     int(name)
        #     valid = False
        # except ValueError:

        # if not valid:
        #     raise ValueError("Cannot accept integer as tile name, even in str form.")

    def unname_tile(self, target: Tile):
        del self._by_name[target.name]
        target._name = None

    @overload
    def __getitem__(self, key: BoxTag) -> Box:
        ...

    @overload
    def __getitem__(self, key: TileTag) -> Tile:
        ...

    @overload
    def __getitem__(self, key: WindowTag) -> Window:
        ...

    @overload
    def __getitem__(self, key: SpaceTag) -> Canvas:
        ...

    @overload
    def __getitem__(self, key: VertexTag) -> Vertex:
        ...

    @overload
    def __getitem__(self, key: EdgeTag) -> Edge:
        ...

    @singledispatchmethod
    def __getitem__(self, key: Tag[T]) -> T:
        raise TypeError(f"Unrecognised key type: {type(key)}")

    @__getitem__.register
    def _(self, key: TileTag) -> Tile:
        if isinstance(key.id, TileId):
            return self._by_type[key.Element][key.id]
        else:
            return self._by_name[key.id]

    @__getitem__.register
    def _(self, key: VertexTag) -> Vertex:
        return self[key.owner].corners[key.role]

    @__getitem__.register
    def _(self, key: EdgeTag) -> Edge:
        return Edge(self[key.a], self[key.b])

    @__getitem__.register
    def _(self, key: BoxTag) -> Box:
        return Box(self[each] for each in key)