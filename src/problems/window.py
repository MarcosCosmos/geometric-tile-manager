from geometry.graph.window import Window
from problems.tile import TileProblem


class WindowProblem(TileProblem):
    box: Window
    def __init__(self, box: Window):
        super().__init__(box)
    @property
    def window(self) -> Window:
        return self.box