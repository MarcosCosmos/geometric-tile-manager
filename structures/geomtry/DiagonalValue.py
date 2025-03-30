import collections
from functools import singledispatchmethod

from structures.geomtry import Axis, CardinalDirection, CardinalValue


class DiagonalValue(collections.namedtuple('_DiagonalValue', tuple(each.snake_case_name for each in Axis))):
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