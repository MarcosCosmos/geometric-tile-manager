from structures.graph import Edge
from structures.problems import StateProblem


class EdgeProblem(StateProblem):
    edge: Edge
    def __init__(self, edge: Edge):
        self.edge = edge