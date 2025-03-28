from __future__ import annotations

import collections
import dataclasses
import operator
from abc import ABC
from dataclasses import dataclass
from enum import Enum
from functools import cache, singledispatchmethod
from types import GenericAlias
from typing import *
import inspect

from structures.geomtry.OrientationEnum import OrientationEnum
from structures.utility.utility import resolve_type_arguments, EnumDataclass, DCEnumT, enum_dataclass

T = TypeVar('T')

class Axis(OrientationEnum):
    @property
    @cache
    def perpendicular(self) -> Axis:
        return Axis(not self.value)

    @property
    @cache
    def directions(self) -> tuple[CardinalDirection, CardinalDirection]:
        return (CardinalDirection((self, False)), CardinalDirection((self, True)))

    HORIZONTAL = False
    VERTICAL = True

class AxisDataclass(EnumDataclass[Axis, T]):
    ...

class Direction(OrientationEnum):
    ...

@enum_dataclass(frozen=True)
class Vector(AxisDataclass[T]):

    def __str__(self):
        return f'Vector({self.horizontal},{self.vertical})'

    @property
    def debug_string(self) -> str:
        return f'({self.horizontal},{self.vertical})'

    def __op(self, other: Vector[T], op: Callable[[Vector[T], Vector[T]], Vector[T]]):
        return Vector(*tuple(op(*each) for each in zip(self, other)))

    def __add__(self, other: Vector[T]):
        return self.__op(other, operator.add)

    def __sub__(self, other: Vector[T]) -> Vector[T]:
        return self.__op(other, operator.sub)

    def __matmul__(self, other: Vector[T]) -> Vector[T]:
        return self.__op(other, operator.mul)

    def __mul__(self, scale: T) -> Vector[T]:
        return Vector(*tuple(each * scale for each in self))

class CardinalValue(NamedTuple):
    """
    enum value for CardinalDirection, in namedtuple form
    """
    axis: Axis
    is_positive: bool

class CardinalDirection(CardinalValue, Direction):
    """
    Negative/positive refers to the sign of unit vectors along the relevant axis. I.e. North and West are negative directions, whilst South and East are positive directions.

    Booleans also used as part of internal logic in this file for accessing enum members by value on demand, but this should not be used outside this file - all relevant logic should be covered by helper properties and stored object members
    """

    unit_vector: Final[Vector[int]]

    def tmp(self, *args, **kwargs):
        print(args, kwargs)

    def __init__(self, *args):
        super().__init__(*args)
        self.unit_vector = Vector._make({
            self.axis: 1 if self.is_positive else -1,
            self.axis.perpendicular: 0
        })

    @property
    @cache
    def opposite(self) -> CardinalDirection:
        return CardinalDirection((self.axis, not self.is_positive))

    @property
    @cache
    def diagonals(self) -> tuple[DiagonalDirection, DiagonalDirection]:
        return tuple(
            DiagonalDirection._make({self.axis: self,self.axis.perpendicular: other})
            for other in self.axis.perpendicular.directions
        )

    @classmethod
    def _make(*args, **kwargs) -> CardinalDirection:
        return CardinalDirection(CardinalValue._make(*args, **kwargs))

    NORTH = (Axis.VERTICAL, False)
    EAST = (Axis.HORIZONTAL, True)
    SOUTH = (Axis.VERTICAL, True)
    WEST = (Axis.HORIZONTAL, False)

    @staticmethod
    def _make(data) -> CardinalDirection:
        return CardinalDirection(CardinalValue._make(data))

class DiagonalValue(collections.namedtuple('_DiagonalValue', tuple(each.snake_case_name for each in Axis))):
    #note: trying to directly inherit NamedTuple fails due to overriding _make.

    """
    Enum value for CardinalDirection, in namedtuple form.
    """
    horizontal: CardinalDirection
    vertical: CardinalDirection

    @singledispatchmethod
    @classmethod
    def _make(cls, data):
        return cls(*data)

    @_make.register(dict)
    @classmethod
    def _(cls, data: dict[Axis, CardinalValue]):
        return cls(*map(data.__getitem__, Axis))

    @singledispatchmethod
    def __getitem__(self, key) -> CardinalDirection:
        return tuple.__getitem__(self, key)

    @__getitem__.register
    def __getitem__(self, key: Axis) -> CardinalDirection:
        return tuple.__getitem__(self, key.value)

class DiagonalDirection(DiagonalValue, Direction):
    """
    Allows construction with a dict[Axis, CardinalDirection] by overriding _make
    """

    @property
    @cache
    def opposite(self):
        return DiagonalDirection._make((self.horizontal.opposite, self.vertical.opposite))

    NORTH_WEST = (CardinalDirection.WEST, CardinalDirection.NORTH)
    NORTH_EAST = (CardinalDirection.EAST, CardinalDirection.NORTH)
    SOUTH_EAST = (CardinalDirection.EAST, CardinalDirection.SOUTH)
    SOUTH_WEST = (CardinalDirection.WEST, CardinalDirection.SOUTH)

    @classmethod
    def _make(cls, *args, **kwargs) -> DiagonalDirection:
        return cls(DiagonalValue._make(*args, **kwargs))

#not creating a whole stub file just for these three hints right now.
class CardinalDataclass(EnumDataclass[CardinalDirection, T]):
    ...

class DiagonalDataclass(EnumDataclass[DiagonalDirection, T]):
    ...

#polutes the namespace because these really should be explicitly reserved words throughout this project
HORIZONTAL = Axis.HORIZONTAL
VERTICAL = Axis.VERTICAL
NORTH = CardinalDirection.NORTH
EAST = CardinalDirection.EAST
SOUTH = CardinalDirection.SOUTH
WEST = CardinalDirection.WEST
NORTH_WEST = DiagonalDirection.NORTH_WEST
NORTH_EAST = DiagonalDirection.NORTH_EAST
SOUTH_EAST = DiagonalDirection.SOUTH_EAST
SOUTH_WEST = DiagonalDirection.SOUTH_WEST

if TYPE_CHECKING:
    @type_check_only
    class AxisDataclass(EnumDataclass[Axis, T]):
        """
        Note: these member annotations exist for linting/autocomplete but are not used at runtime, since the dataclass fields come directly from the enum itself.
        The real class is otherwise identical.
        """
        horizontal: T
        vertical: T

    @type_check_only
    class CardinalDataclass(EnumDataclass[CardinalDirection, T]):
        """
        Note: these member annotations exist for user linting/autocomplete but are not used at runtime, since the real dataclass fields come directly from the enum itself.
        The real class is otherwise identical.
        """
        north: T
        east: T
        south: T
        west: T

    @type_check_only
    class DiagonalDataclass(EnumDataclass[DiagonalDirection, T]):
        """
        Note: these member annotations exist for user linting/autocomplete but are not used at runtime, since the dataclass fields come directly from the enum itself.
        The real class is otherwise identical.
        """
        north_west: T
        north_east: T
        south_east: T
        south_west: T
