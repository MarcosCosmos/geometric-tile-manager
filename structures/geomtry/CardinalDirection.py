from structures.geomtry import Axis, CardinalValue, DiagonalDirection, Direction, Vector

from functools import cache
from typing import Final


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