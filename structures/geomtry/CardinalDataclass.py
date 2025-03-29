from typing import TYPE_CHECKING, TypeVar, type_check_only

from structures.geomtry import CardinalDirection
from utility import EnumDataclass


T = TypeVar('T')

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