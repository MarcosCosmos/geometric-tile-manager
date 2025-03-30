from __future__ import annotations

from structures.navigation import ExteriorTileNeighbourhood


class NarrowTileNeighbourhood(ExteriorTileNeighbourhood):
    """
    Considers only those Tiles that are strictly within the span of the owner.

    This is the default neighbourhood used for graph traversal.
    """
    _START_INDEX= -1
    _END_INDEX = 0
