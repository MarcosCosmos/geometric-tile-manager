from structures.graph.Tile import Tile
from structures.graph.WindowTag import WindowTag


class Window(Tile):
    """
    Concrete, indivisible Tiles representing the geometry of an on-screen object (presumably a program window).

    Note:
        Theoretically, a Window could hold a nested Wall, given suitable polymorphic abstraction of size checks etc.
        However, for now such ideas are out of scope.
    """

    def generate_tag(self) -> WindowTag:
        return WindowTag(self._name if self._name is not None else self.id)