from functools import cache
from typing import TYPE_CHECKING, TypeVar

from geometry.orientation import Orientation
from utility.enum_data import EnumDataclass

if TYPE_CHECKING:
    from geometry.direction.cardinal import CardinalDirection

T = TypeVar('T')

class Axis(Orientation):
    @property
    @cache
    def perpendicular(self) -> 'Axis':
        return Axis(not self.value)

    @property
    @cache
    def directions(self) -> tuple['CardinalDirection', 'CardinalDirection']:
        from geometry.direction.cardinal import CardinalDirection
        return CardinalDirection((self, False)), CardinalDirection((self, True))

    HORIZONTAL = False
    VERTICAL = True

class AxisDataclass(EnumDataclass[Axis, T]):
    ...

if TYPE_CHECKING:
    from typing import type_check_only
    @type_check_only
    class AxisDataclass(EnumDataclass[Axis, T]):
        """
        Note: these member annotations exist for linting/autocomplete but are not used at runtime, since the dataclass fields come directly from the enum itself.
        The real class is otherwise identical.
        """
        horizontal: T
        vertical: T