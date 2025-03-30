from geometry.axis import Axis
from geometry.direction.cardinal import CardinalDirection
from geometry.direction.diagonal import DiagonalDirection

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

__all__ = ["HORIZONTAL", "VERTICAL", "NORTH", "EAST", "SOUTH", "WEST", "NORTH_EAST", "NORTH_WEST", "SOUTH_WEST", "SOUTH_EAST"]