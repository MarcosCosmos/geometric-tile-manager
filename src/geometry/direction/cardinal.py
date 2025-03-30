from functools import cache
from typing import TYPE_CHECKING, TypeVar, type_check_only, Final, NamedTuple

from geometry.axis import Axis
from geometry.direction.diagonal import DiagonalDirection
from geometry.direction import Direction
from geometry.vector import Vector
from utility import EnumDataclass

T = TypeVar('T')


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
    def opposite(self) -> 'CardinalDirection':
        return CardinalDirection((self.axis, not self.is_positive))

    @property
    @cache
    def diagonals(self) -> tuple[DiagonalDirection, DiagonalDirection]:
        return tuple(
            DiagonalDirection._make({self.axis: self,self.axis.perpendicular: other})
            for other in self.axis.perpendicular.directions
        )

    @classmethod
    def _make(cls, *args, **kwargs) -> 'CardinalDirection':
        return cls(CardinalValue._make(*args, **kwargs))

    NORTH = (Axis.VERTICAL, False)
    EAST = (Axis.HORIZONTAL, True)
    SOUTH = (Axis.VERTICAL, True)
    WEST = (Axis.HORIZONTAL, False)

    # @staticmethod
    # def _make(data) -> 'CardinalDirection':
    #     return CardinalDirection(CardinalValue._make(data))


class CardinalDataclass(EnumDataclass[CardinalDirection, T]):
    ...


if TYPE_CHECKING:
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