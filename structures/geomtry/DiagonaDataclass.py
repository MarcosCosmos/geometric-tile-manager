from __future__ import annotations
from typing import TYPE_CHECKING, TypeVar, type_check_only

from structures.geomtry.DiagonalDirection import DiagonalDirection
from structures.utility import EnumDataclass

T = TypeVar('T')

#not creating a whole stub file just for these three hints right now.

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
