from __future__ import annotations

from typing import Sequence

from procedures.examination import is_divided
from structures.navigation import TileNeighbourhood
from structures.geomtry import CardinalDirection
from structures.graph import Wall, Tile


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
