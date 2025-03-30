from __future__ import annotations

import collections
from functools import cache, singledispatchmethod
from typing import TYPE_CHECKING, TypeVar, type_check_only

from geometry.axis import Axis
from geometry.direction.cardinal import CardinalDirection, CardinalValue
from geometry.direction import Direction
from utility import EnumDataclass

T = TypeVar('T')

_tuplearg = tuple(each.snake_case_name for each in Axis)

class DiagonalValue(collections.namedtuple('_DiagonalValue', _tuplearg)):
    #note: trying to directly inherit NamedTuple fails due to overriding _make.

    """
    Enum value for DiagonalDirection, in namedtuple form.
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
    def _make(cls, *args, **kwargs) -> 'DiagonalDirection':
        return cls(DiagonalValue._make(*args, **kwargs))


class DiagonalDataclass(EnumDataclass[DiagonalDirection, T]):
    ...

if TYPE_CHECKING:
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