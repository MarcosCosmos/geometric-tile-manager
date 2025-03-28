from structures.geomtry import CardinalDirection, DiagonalValue
from structures.geomtry.Direction import Direction


from functools import cache


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