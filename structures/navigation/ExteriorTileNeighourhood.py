from __future__ import annotations

from typing import ClassVar, Sequence

from structures.navigation import TileNeighbourhood
from structures.geomtry import CardinalDirection
from structures.graph import Tile


class ExteriorTileNeighbourhood(TileNeighbourhood):
    """
    Defines the exterior neighbouring tiles in each cardinal direction. More importantly, it determines the minimum set of tiles that would mandatory be affected by moving an edge of the tile. This minimum set can be expanded heuristically to achieve more advanced behaviour.

    Each side is computed on-demand from vertex neighbourhoods.

    Used heavily for determining Boxes for other actions.
    - Might be more efficient technically to directly compute the edge points as it would not require iteration.
    -- For now however, it is more desirable to be declarative/provide more exposition in the initial prototype and as implemented/backup documentation in future if the more efficient alternative is eventually preferred.
    -- This can be decided or interpreted in multiple ways and is therefore configurable.
    """

    """
        Defines the first index, among the neighbours of the start corner of a side, to include in the neighbourhood. This, along with the end index, is used to control tiebreaker scenarios when a corner vertex is neighboured by 2 other vertices due to a gap.
        
        Since the number of neighbours will vary, 
        You can use negative values to 
        A value of -1 
    """
    _START_INDEX: ClassVar[int]
    _END_INDEX: ClassVar[int]

    def __getitem__(self, side: CardinalDirection) -> Sequence[Tile]:
        """
        Finds the tiles associated with the vertices adjacent to nodes within the axis-aligned range defined by a side's edge. This automatically captures any tiles which happen to straddle a corner since at least one of the vertices would have to be within that range regardless.
        :param sideName:
        :return:
        """
        target_edge = self.owner.sides[side]
        forwardDirection = side.axis.perpendicular.directions[-1]

        # note: if the vertex lies between two other vertices on a side it neccessarily means it would have both as neighbours they would otherwise not be within the gap range.
        current = target_edge.a.neighbours[side][self._START_INDEX]

        end = target_edge[-1].neighbours[side][self._END_INDEX]

        result = []
        result.append(current.owner)

        while current is not end:
            current = current.neighbours[forwardDirection]
            if current.owner is not result[-1]:
                result.append(current.owner)

        return result
