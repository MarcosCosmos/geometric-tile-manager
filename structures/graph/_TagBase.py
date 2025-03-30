from typing import Generic, TypeVar

T = TypeVar('T')
class _TagBase(Generic[T]):
    """
    Exists purely to allow Tag to inherit something other than itself for the purpose of Element type resolution.
    """
    ...