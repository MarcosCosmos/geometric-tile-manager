from structures.graph.Tile import Tile
from structures.graph.CanvasTag import SpaceTag


class Canvas(Tile):
    """
    A special Tile defining the boundaries of a space to place tiles within.

    - Unlike Windows, the corners of Canvass are only directly connected on the interior if there are no Tiles within, otherwise they connect to the tile nearest to that corner.
    -- In other words Canvass are allowed to be divided in the sense that other Tiles are placed on them, but Windows are not.
    - Canvass act as sentinel Tiles defining the boundaries/dimensions of a tiling canvas.
    -- Additionally, Canvass may be connected to each other on the exterior, to model navigation through multiple desktops.
    --- Accordingly, a sentinel is NOT equivalent to None or to the end of navigation.

    Rules:
    - No particular constraints are placed on distances or positioning between canvass, except that their corners cannot be out of order.
    - There are no perimeter margins, each corner of a canvas must have the same position at the corner of exactly one Window (unless there are no Windows, of course.
    - Canvass are implicitly their own canvas/container

    Note:
        Canvass may have neighbours for the purpose of navigation (e.g. across monitors), but their vertices are still sentinels. o None.
    """

    # def __init__(self, id: TileId, north_west: Vector, north_east: Vector, south_east: Vector, south_west: Vector):
    #     super().__init__(id, north_west, north_east, south_east, south_west)

    # def generate_tag(self) -> Canvas.Tag:
    #     return super().generate_tag()

    def generate_tag(self) -> SpaceTag:
        return SpaceTag(self._name if self._name is not None else self.id)