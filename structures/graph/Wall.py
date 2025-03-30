from structures.graph.Tile import Tile
from structures.graph.WallTag import WallTag


class Wall(Tile):
    """
    A special Tile defining the boundaries of a space to place tiles within.

    - Unlike Windows, the corners of Walls are only directly connected on the interior if there are no Tiles within, otherwise they connect to the tile nearest to that corner.
    -- In other words Walls are allowed to be divided in the sense that other Tiles are placed on them, but Windows are not.
    - Walls act as sentinel Tiles defining the boundaries/dimensions of a tiling canvas.
    -- Additionally, Walls may be connected to each other on the exterior, to model navigation through multiple desktops.
    --- Accordingly, a sentinel is NOT equivalent to None or to the end of navigation.

    Rules:
    - No particular constraints are placed on distances or positioning between walls, except that their corners cannot be out of order.
    - There are no perimeter margins, each corner of a wall must have the same position at the corner of exactly one Window (unless there are no Windows, of course.
    - Walls are implicitly their own wall/container

    Note:
        Walls may have neighbours for the purpose of navigation (e.g. across monitors), but their vertices are still sentinels. o None.
    """

    # def __init__(self, id: TileId, north_west: Vector, north_east: Vector, south_east: Vector, south_west: Vector):
    #     super().__init__(id, north_west, north_east, south_east, south_west)

    # def generate_tag(self) -> Wall.Tag:
    #     return super().generate_tag()

    def generate_tag(self) -> WallTag:
        return WallTag(self._name if self._name is not None else self.id)