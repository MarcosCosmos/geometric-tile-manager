from __future__ import annotations

from typing import ClassVar, Sequence, Final, MutableSequence

from geometry.direction.cardinal import CardinalDataclass, CardinalDirection
from geometry.graph.canvas import Canvas
from geometry.graph.tile import Tile
from utility.enum_data import enum_dataclass


@enum_dataclass
class VertexNeighbourhood(CardinalDataclass[MutableSequence['Vertex']]):
    """
    Slightly different from ordinary definitions, neighbourhoods here are directionally subdivided as they must be axis aligned, ordered, and we need to know the neighbours in both directions if a point lies between two neighbours along an axis.
    Neighbourhoods, unlike boxes, are often subject to updates and do not need to be treated as immutable.


    Neighbourhoods are initialised to be empty on all sides, then become populated after the vertex is created, due to the need for circular reference.

    Note: After proper population, the neighbours may be ONLY be None for the outgoing directions of a Canvas.

    Note: the possible lengths on each side for vertex neighbours must be in the range [0,2], where 0 is only possible for the outward sides of a Canvas and 2 is only possible if the vertex is between the corners of two corners of the associated neighbouring tile(s)
    """

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


class InteriorCanvasNeighbourhood(TileNeighbourhood):
    """
    Unlike exterior neighbourhoods, this defines available tiles on interior of the canvas, facing inward from the chosen side.
    """
    _owner: Canvas

    def __init__(self, owner: Canvas):
        super().__init__(owner)

    def __getitem__(self, side: CardinalDirection) -> Sequence[Tile]:
        from procedures.examination import is_divided
        """
        Finds the tiles associated with the vertices adjacent to nodes within the axis-aligned range defined by a side's edge. This automatically captures any tiles which happen to straddle a corner since at least one of the vertices would have to be within that range regardless.
        :param sideName:
        :return:
        """
        if not is_divided(self._owner):
            return [] #if the canvas is unbroken it means there are no contents.

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
        return self.__getitem__(CardinalDirection.NORTH)

    @property
    def east(self) -> Sequence[Tile]:
        return self.__getitem__(CardinalDirection.EAST)

    @property
    def south(self) -> Sequence[Tile]:
        return self.__getitem__(CardinalDirection.SOUTH)

    @property
    def west(self) -> Sequence[Tile]:
        return self.__getitem__(CardinalDirection.WEST)


class NarrowTileNeighbourhood(ExteriorTileNeighbourhood):
    """
    Considers only those Tiles that are strictly within the span of the owner.

    This is the default neighbourhood used for graph traversal.
    """
    _START_INDEX = -1
    _END_INDEX = 0


class WideTileNeighbourhood(ExteriorTileNeighbourhood):
    """
    Considers both adjacent Tiles in the event that a corner lies on the margin between them.

    This is the default neighbourhood used for manipulations as it ensures that that manipulations affect a single axis as much as is possible.
    """

    _START_INDEX = 0
    _END_INDEX = -1