from geometry.graph.box import Box
from problems.state import StateProblem


class BoxProblem(StateProblem):
    box: Box
    def __init__(self, box: Box):
        self.box = box


class BoxTooLargeForConstraintsProblem(BoxProblem):
    @property
    def description(self) -> str:
        return f'The box {self.box.debug_string} was too small to achieve all desired max segment sizes.'


class BoxTooSmallForConstraintsProblem(BoxProblem):
    @property
    def description(self) -> str:
        return f'The box {self.box.debug_string} was too small to achieve all desired minimum segment sizes.'


class BoxTooSmallForMarginsProblem(BoxProblem):
    @property
    def description(self) -> str:
        return f'The box {self.box} was too small to allocate margins.'
