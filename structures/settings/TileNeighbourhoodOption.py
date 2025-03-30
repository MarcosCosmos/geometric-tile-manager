from structures.navigation import NarrowTileNeighbourhood, TileNeighbourhood, WideTileNeighbourhood


class TileNeighbourhoodOption(TileNeighbourhood):
    NARROW = NarrowTileNeighbourhood
    WIDE = WideTileNeighbourhood