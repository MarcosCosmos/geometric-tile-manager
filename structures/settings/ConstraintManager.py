from typing import Final


# class ConstraintManager:
#     """
#     Storage for actively set constraints
#     """
#     _parent: Final[SettingManager]
#     minimum_window_size: Final[DefaultedDict[Window, Point[int]]]
#
#     def __init__(self, parent: SettingManager):
#         self._parent = parent
#         self.minimum_window_size = DefaultedDict(lambda: self._parent.static_config.constraints.minimum_window_size)
#
#     @property
#     def window_margin(self) -> int:
#         return self._parent.static_config.constraints.window_margin
#
#     @property
#     def window_distance(self) -> int:
#         return self._parent.static_config.constraints.window_margin + 1