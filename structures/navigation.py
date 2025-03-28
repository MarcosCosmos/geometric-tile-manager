from __future__ import annotations
from typing import Final, Sequence, ClassVar

from procedures.examination import is_divided
from structures.geomtry.CardinalDirection import CardinalDirection
from structures.graph import Tile, Wall


class TileNeighbourhood:
    owner: Final[Tile]
    def __init__(self, owner: Tile):
        self.owner = owner

class ExteriorTileNeighbourhood(TileNeighbourhood):
    """
    Defines the exterior neighbouring tiles in each cardinal direction. More importantly, it determines the minimum set of tiles that would mandatory be affected by moving an edge of the tile. This minimum set can be expanded heuristically to achieve more advanced behaviour.

    Each side is computed on-demand from vertex neighbourhoods.

    Used heavily for determining Boxes for other actions.
    - Might be more efficient technically to directly compute the edge points as it would not require iteration.
    -- For now however, it is more desirable to be declarative/provide more exposition in the initial prototype and as implemented/backup documentation in future if the more efficient alternative is eventually preferred.
    -- This can be decided or interpreted in multiple ways and is therefore configurable.
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

        # note: if the vertex lies between two other vertices on a side it neccessarily means it would push both as they would otherwise not be within the gap range.
        current = target_edge[0].neighbours[side][self._START_INDEX]

        end = target_edge[-1].neighbours[side][self._END_INDEX]

        result = []
        result.append(current.owner)

        while current is not end:
            current = current.neighbours[forwardDirection]
            if current.owner is not result[-1]:
                result.append(current.owner)

        return result

class NarrowTileNeighbourhood(ExteriorTileNeighbourhood):
    """
    Considers only those Tiles that are strictly within the span of the owner.

    This is the default neighbourhood used for graph traversal.
    """
    _START_INDEX= -1
    _END_INDEX = 0


class WideTileNeighbourhood(ExteriorTileNeighbourhood):
    """
    Considers both adjacent Tiles in the event that a corner lies on the margin between them.

    This is the default neighbourhood used for manipulations as it ensures that that manipulations affect a single axis as much as is possible.
    """

    _START_INDEX = 0
    _END_INDEX = -1

class InteriorWallNeighbourhood(TileNeighbourhood):
    """
    Unlike exterior neighbourhoods, this defines available tiles on interior of the wall, facing inward from the chosen side.
    """
    _owner: Wall

    def __init__(self, owner: Wall):
        super().__init__(owner)

    def __getitem__(self, side: CardinalDirection) -> Sequence[Tile]:
        """
        Finds the tiles associated with the vertices adjacent to nodes within the axis-aligned range defined by a side's edge. This automatically captures any tiles which happen to straddle a corner since at least one of the vertices would have to be within that range regardless.
        :param sideName:
        :return:
        """
        if not is_divided(self._owner):
            return [] #if the wall is unbroken it means there are no contents.

        targetCorners = self.owner.sides(side.diagonals)
        forwardDirection = side.axis.perpendicular.directions[-1]

        # note: if the vertex lies between two other vertices on a side it necessarily means it would push both as they would otherwise not be within the gap range.
        current = targetCorners[0].neighbours[side.opposite][0]

        end = targetCorners[1].neighbours[side.opposite][-1]

        result = []
        result.append(current.owner)

        while current is not end:
            current = current.neighbours[forwardDirection]
            if current.owner is not result[-1]:
                result.append(current.owner)

        return result

    @property
    def north(self) -> Sequence[Tile]:
        return self._computeSide(CardinalDirection.NORTH)

    @property
    def east(self) -> Sequence[Tile]:
        return self._computeSide(CardinalDirection.EAST)

    @property
    def south(self) -> Sequence[Tile]:
        return self._computeSide(CardinalDirection.SOUTH)

    @property
    def west(self) -> Sequence[Tile]:
        return self._computeSide(CardinalDirection.WEST)