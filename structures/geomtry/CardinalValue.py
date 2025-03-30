from typing import NamedTuple
from structures.geomtry import Axis

class CardinalValue(NamedTuple):
    """
    enum value for CardinalDirection, in namedtuple form
    """
    axis: Axis
    is_positive: bool