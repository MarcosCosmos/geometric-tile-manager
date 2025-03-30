from __future__ import annotations

from structures.navigation import ExteriorTileNeighbourhood


class WideTileNeighbourhood(ExteriorTileNeighbourhood):
    """
    Considers both adjacent Tiles in the event that a corner lies on the margin between them.

    This is the default neighbourhood used for manipulations as it ensures that that manipulations affect a single axis as much as is possible.
    """

    _START_INDEX = 0
    _END_INDEX = -1
