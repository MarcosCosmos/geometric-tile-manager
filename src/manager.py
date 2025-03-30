from settings import Settings
from geometry.graph.registry import TileRegistry


class GeometricTileManager:
    """
    Root object for an instance of the Geometric Tile Manager.
    Includes the TileGraph and SettingsManager objects.

    Whilst many procedures relate to this object (e.g. as its first parameter), they are written to be stand-alone rather than member methods for now, as they are intended to be modular. In future, additional modules such as an IPC handler and individual Manipulation/navigation managers may also be added to store current states for navigation and manipulation selections.
    """

    graph: TileRegistry
    settings: Settings

    def __init__(self):
        self.graph = TileRegistry()
        self.settings = Settings()