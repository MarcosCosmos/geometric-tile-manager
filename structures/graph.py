from __future__ import annotations

import dataclasses
import warnings
import itertools
from abc import ABC, abstractmethod, ABCMeta
from collections import MutableMapping
from ctypes import Union
from dataclasses import dataclass
from enum import Enum, unique
from functools import cache, singledispatch, singledispatchmethod

import parse as ps
from typing import NewType, Sequence, Final, Optional, Iterator, Iterable, MutableSequence, TypeVar, Generic, \
    NamedTuple, overload, Type, Mapping, ClassVar, get_args, get_origin

from structures.geomtry.Axis import Axis
from structures.geomtry.CardinalDirection import CardinalDirection
from structures.geomtry.DiagonalDirection import DiagonalDirection
from structures.geomtry.Vector import Vector
from structures.geomtry.DiagonaDataclass import *

T = TypeVar('T')

def parse(format: str, text: str) -> Sequence[str]:
    result = ps.parse(format, text)
    return [result.named[each[0]] for each in sorted(result.spans.items(), key=lambda x: x[1])]

class _TagBase(Generic[T]):
    """
    Exists purely to allow Tag to to inherit something other than itself for the purpose of Element type resolution.
    """
    ...
class TagMeta(ABCMeta):
    def __new__(mcls, name, bases, namespace, **kwargs):
        # get_args
        cls = super().__new__(mcls, name, bases, namespace, **kwargs)
        (cls.Element,) = resolve_type_arguments(_TagBase, GenericAlias(cls, cls.__parameters__))
        cls.__annotations__

        return cls

class Tag(_TagBase[T], metaclass=TagMeta):
    """
    Tags help external users to refer to graph elements such as Tiles, Vertices, and Edges e.g. via IPC.
    They are composite objects describing the element's type and role.
    -- Tiles are the principle basis for uniqueness. They receive numerical ids. In future, custom string strings might be allowed as ids.
    --- Note: other properties like window title etc are not part of identification here, that is left to other libraries to manage in combination with what is provided here.
    -- Vertices are identified by their owning Tile and the corner they correspond to.
    -- Other items, like general Boxes and Edges are defined by the vertices that bound them.
    - Although not all elements are actionable, it is generally valid for a user to specify an arbitrary element as a search term to find a suitable element for a given action, which is generally the smallest containing element of the correct type.

    They can be converted to strings and will be parseable in future.


    todo: equality, hash, decode, etc.
    """

    Element: ClassVar[Type[T]]
    format: ClassVar[str]

    def __str__(self):
        return f'{self.Element.__name__}({self.inner_str})'

    @property
    def inner_str(self) -> str:
        return self.format.format(self=self)

    def __repr__(self):
        return str(self)

    @staticmethod
    def parse(text: str) -> Tag:
        (target_name, inner_text) = ps.parse('{}({})', text).fixed
        for each_cls in Tag.__subclasses__():
            if target_name == each_cls.Element.__name__:
                return each_cls.parse(inner_text)


class TaggableElement:
    """
    We can't use this class with Edge, which a namedtuple and therefore cannot be an element, but it already have a suitable repr
    :param type:
    :return:
    """

    @abstractmethod
    def generate_tag(self) -> Tag:
        ...

    def __str__(self) -> str:
        return str(self.generate_tag())

    def __repr__(self) -> str:
        return f'<{self.__class__.__module__}.{self.__class__.__name__} at {hex(id(self))}: {self.generate_tag()}>'

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
    location: Vector[int] #not optional, so Windows should only be created via algorthms - walls could possibly simulate free floating layers? or an extension of walls.
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

@enum_dataclass
class VertexNeighbourhood(CardinalDataclass[MutableSequence[Vertex]]):
    """
    Slightly different from ordinary definitions, neighbourhoods here are directionally subdivided as they must be axis aligned, ordered, and we need to know the neighbours in both directions if a point lies between two neighbours along an axis.
    Neighbourhoods, unlike boxes, are often subject to updates and do not need to be treated as immutable.


    Neighbourhoods are initialised to be empty on all sides, then become populated after the vertex is created, due to the need for circular reference.

    Note: After proper population, the neighbours may be ONLY be None for the outgoing directions of a Wall.

    Note: the possible lengths on each side for vertex neighbours must be in the range [0,2], where 0 is only possible for the outward sides of a Wall and 2 is only possible if the vertex is between the corners of two corners of the associated neighbouring tile(s)
    """

@dataclass
class VertexTag(Tag[Vertex]):
    format='{self.owner.inner_str}.{self.role.snake_case_name}'
    owner: TileTag
    role: DiagonalDirection

    @staticmethod
    def parse(text: str) -> VertexTag:
        owner, role = parse(VertexTag.format, text)
        return VertexTag(TileTag.parse(owner), DiagonalDirection[role.upper()])

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
    def parse(text: str) -> EdgeTag:
        return EdgeTag(*map(VertexTag.parse, parse(EdgeTag.format, text)))

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
        _owner: Final[Box]
        def __init__(self, owner: Box):
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
    def parse(text: str) -> BoxTag:
        return BoxTag(*map(VertexTag.parse, parse(BoxTag.format, text)))

class IndependantBox(Box):
    """
    Base class for non-tile Boxes. Required to be a valid box, but can cover one or more other boxes.

    These boxes are generally temporary and are primarily used for to store computational results as part of manipulation processes
    """

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
        return isinstance(self, Wall)

class TileId(NamedTuple):
    """
    Wrapper for internally generated tile ids to prevent name conflicts with e.g. actual OS window ids, which may be commonly used as names in practice.
    """
    number: int
    def __str__(self):
        return f'<{self.number}>'

@dataclass
class TileTag(Tag[Tile]):
    format='{self.id}'
    id: TileId | str

    @staticmethod
    def parse(id: str) -> TileTag:
        match = ps.parse("<{:d}>", id)
        if match is None:
            return TileTag(id)
        else:
            return TileTag(TileId(*match.fixed))

class Window(Tile):
    """
    Concrete, indivisible Tiles representing the geometry of an on-screen object (presumably a program window).

    Note:
        Theoretically, a Window could hold a nested Wall, given suitable polymorphic abstraction of size checks etc.
        However, for now such ideas are out of scope.
    """

    def generate_tag(self) -> WindowTag:
        return WindowTag(self._name if self._name is not None else self.id)

class WindowTag(TileTag, Tag[Window]):
    ...

class Wall(Tile):
    """
    A special Tile defining the boundaries of a space to place tiles within.

    - Unlike Windows, the corners of Walls are only directly connected on the interior if there are no Tiles within, otherwise they connect to the tile nearest to that corner.
    -- In other words Walls are allowed to be divided in the sense that other Tiles are placed on them, but Windows are not.
    - Walls act as sentinel Tiles defining the boundaries/dimensions of a tiling canvas.
    -- Additionally, Walls may be connected to each other on the exterior, to model navigation through multiple desktops.
    --- Accordingly, a sentinel is NOT equivalent to None or to the end of navigation.

    Rules:
    - No particular constraints are placed on distances or positioning between walls, except that their corners cannot be out of order.
    - There are no perimeter margins, each corner of a wall must have the same position at the corner of exactly one Window (unless there are no Windows, of course.
    - Walls are implicitly their own wall/container

    Note:
        Walls may have neighbours for the purpose of navigation (e.g. across monitors), but their vertices are still sentinels. o None.
    """

    # def __init__(self, id: TileId, north_west: Vector, north_east: Vector, south_east: Vector, south_west: Vector):
    #     super().__init__(id, north_west, north_east, south_east, south_west)

    # def generate_tag(self) -> Wall.Tag:
    #     return super().generate_tag()

    def generate_tag(self) -> WallTag:
        return WallTag(self._name if self._name is not None else self.id)

class WallTag(TileTag, Tag[Wall]):
    ...

TileT = TypeVar('TileT', bound=Tile)

class TileRegistry:
    """
    TileRegistries are Tile factories and provide storage/lookup functionality.

    It also acts as a factory for all tiles, to ensure that it maintains access

    Rules for uniquely identifying and describing graph objects:
    - Wall ids are distinct from Window Ids, so the full descriptor for a tile must include its type.
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
            Wall: {}
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
        :param name: cannot be convertable to a TileId (that is, if it a string, int(name) but fail.
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
    def __getitem__(self, key: WallTag) -> Wall:
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



