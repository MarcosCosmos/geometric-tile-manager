from geometry.graph.edge import Edge
from problems.state import StateProblem


class EdgeProblem(StateProblem):
    edge: Edge
    def __init__(self, edge: Edge):
        self.edge = edge


class BrokenEdgeProblem(EdgeProblem):
    @property
    def description(self) -> str:
        return f'The vertices along edge {self.edge.debug_string} must be the only neighbours in the directions facing one another, but they are not.'


class NotAxisAlignedProblem(EdgeProblem):
    @property
    def description(self) -> str:
        return f'{self.edge.debug_string} is aligned to either axis.'


class OutOfOrderProblem(EdgeProblem):
    def __init__(self, edge, axis):
        super().__init__(edge)
        self.axis = axis
    @property
    def description(self) -> str:
        return f'The vertices of {self.edge.debug_string} are out of order along the {self.axis} axis.'
