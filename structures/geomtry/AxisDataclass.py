
from typing import TYPE_CHECKING, TypeVar, type_check_only
from structures.geomtry import Axis
from structures.utility import EnumDataclass

T = TypeVar('T')
class AxisDataclass(EnumDataclass[Axis, T]):
    ...

if TYPE_CHECKING:
    @type_check_only
    class AxisDataclass(EnumDataclass[Axis, T]):
        """
        Note: these member annotations exist for linting/autocomplete but are not used at runtime, since the dataclass fields come directly from the enum itself.
        The real class is otherwise identical.
        """
        horizontal: T
        vertical: T