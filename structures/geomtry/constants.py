
#polutes the namespace because these really should be explicitly reserved words throughout this project
from structures.geomtry import Axis
from structures.geomtry import CardinalDirection
from structures.geomtry import DiagonalDirection


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