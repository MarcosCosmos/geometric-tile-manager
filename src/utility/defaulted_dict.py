from typing import Callable, TypeVar


KT = TypeVar('KT')
VT = TypeVar('VT')
class DefaultedDict(dict[KT, VT]):
    """
    Intended to initially provide default values from the configuration, but allow them to be overridden on a per-tile basis.
    Slightly different to dict.setdefault, which would need to be set per-tile.
    """
    _fallback_fn: Callable[[], VT]
    def __init__(self, fallback_fn, seq=None, **kwargs):
        super().__init__(seq=seq,**kwargs)
        self._fallback_fn = fallback_fn

    def __get_item__(self, key) -> VT:
        try:
            return super().__getitem__(key)
        except KeyError:
            return self._fallback_fn()