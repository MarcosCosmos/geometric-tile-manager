from __future__ import annotations
from typing import Type, TypeVar, Optional

from structures.navigation import TileNeighbourhood, NarrowTileNeighbourhood, WideTileNeighbourhood

T = TypeVar('T')

class TileNeighbourhoodOption(TileNeighbourhood):
    NARROW = NarrowTileNeighbourhood
    WIDE = WideTileNeighbourhood

class DirectionSensitiveTiebreaker:
    EXTREME_SYMMETRICAL_NORTH_WEST_AND_SOUTH_EAST = lambda direction, options: options[0 if direction.is_positive else -1]

class StaticConfiguration:
    """
    Storage for configuration options that control various logic.

    These options are static/global/default configurations, some of which can be overridden on a per-tile basis.

    Options are divided into subcategories to separate options which have similar purposes in different contexts (e.g. TileNeighbourhoods)

    Note: for now, new configurations should always and exclusively be created by either calling the Configuration() constructuctor, to base off of the default constructor, or else by taking a deep-copy of another configuration. In future, yaml support might be provided.
    """
    class Navigation:
        tile_neighbourhood: Type[TileNeighbourhood]
        tiebreaker: DirectionSensitiveTiebreaker.EXTREME_SYMMETRICAL_NORTH_WEST_AND_SOUTH_EAST
        def __init__(self):
            self.tile_neighbourhood = TileNeighbourhoodOption.NARROW


    class Manipulation:
        ...

    class Render:
        #todo: for actual usage this will default to a transparent background to overlay over the managed windows.
        bg_color: str

        #fallback colours
        default_tile_border_color: str
        default_tile_border_width: int
        _wall_border_color: Optional[str]
        _wall_border_width: Optional[int]
        _window_border_color: Optional[str]
        _window_border_width: Optional[int]

        edge_color: str
        edge_dash: tuple[int,int]
        sentinel_edge_color: str

        #todo: add specific colouring for targetted edges, etc.

        def __init__(self):
            self.bg_color = 'white'
            self.default_tile_border_color = 'black'
            self.default_tile_border_width = 1
            self._wall_border_color = 'darkgrey'
            self._wall_border_width = None
            self._window_border_color = None
            self._window_border_width = None
            self.edge_color = 'red'
            self.edge_dash = (2,4)
            self.sentinel_edge_color = 'blue'

        @property
        def wall_border_color(self) -> str:
            return self._wall_border_color if self._wall_border_color is not None else self.default_tile_border_color

        @property
        def window_border_color(self) -> str:
            return self._window_border_color if self._window_border_color is not None else self.default_tile_border_color

        @property
        def wall_border_width(self) -> int:
            return self._wall_border_width if self._wall_border_width is not None else self.default_tile_border_width
        @property
        def window_border_width(self) -> int:
            return self._window_border_width if self._window_border_width is not None else self.default_tile_border_width

    class Contraints:
        window_margin: int
        # minimum_window_size: Point[int]
        def __init__(self):
            self.window_margin = 0 #Note: a tile margin of 0 means a minimum distance between tiles of 1, as the margin is the additional space not occupied by the tiles themselves.
            # self.default_minimum_tile_size = Point(1,1)

    constraints: Contraints
    navigation: Navigation
    manipulation: Manipulation
    render: Render

    def __init__(self):
        self.constraints = StaticConfiguration.Contraints()
        self.navigation = StaticConfiguration.Navigation()
        self.manipulation = StaticConfiguration.Manipulation()
        self.render = StaticConfiguration.Render()

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
#     # @property
#     # def window_distance(self) -> int:
#     #     return self._parent.static_config.constraints.window_margin + 1

class Settings:
    static_config: StaticConfiguration
    # active_constraints: ConstraintManager

    def __init__(self):
        self.static_config = StaticConfiguration()
