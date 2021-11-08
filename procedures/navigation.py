from typing import Optional

from procedures.examination import is_divided
from structures.graph import Tile, CardinalDirection, Wall
from structures.manager import GeometricTileManager
from structures.navigation import InteriorWallNeighbourhood

"""
Basic navigation procedures related to user actions (like changing window focus).
Not to be confused with the selection module/file, which determines actionable regions for the purpose of manipulations.
"""
def next_tile(manager: GeometricTileManager, initial: Tile, direction: CardinalDirection) -> Optional[Tile]:
    """
    When multiple options are available, option furthest towards the top-left/bottom-right corner will be taken, depend on which corner the input direction faces.
    - This maximises the likelihood that reversing a navigation will reach the previous Tile.
    """

    candidates = manager.settings.static_config.navigation.tile_neighbourhood(initial)[direction]

    if len(candidates) == 0:
        return None
    else:
        return manager.settings.static_config.navigation.tiebreaker(candidates)


def next_undivided_tile(manager: GeometricTileManager, initial: Tile, direction: CardinalDirection) -> Optional[Tile]:
    """
    This variant is limited to Tiles whose corners are all each other's nearest neighbours (all tiles, but only empty walls).
    - This matches the most common conditions for focusable content in tiling window managers.

    When multiple options are available, option furthest towards the top-left/bottom-right corner will be taken, depend on which corner the input direction faces.
    -This maximises the likelihood that reversing a navigation will reach the previous Tile.
    """
    result = next_tile(manager, initial, direction)

    if result is None:
        return None

    if not initial.is_sentinel and result.is_sentinel:
        #this means we are moving from a Window to find its containing wall, so we want to skip to the next wall, if any
        result = next_tile(manager, result, direction)

        if result is None:
            return None

    #isinstance actually makes more sense that is_sentinel here
    if isinstance(result, Wall) and is_divided(result):
        candidates = InteriorWallNeighbourhood(result)[direction]
        result = manager.settings.static_config.navigation.tiebreaker(candidates)

    return result

