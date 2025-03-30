from structures.geomtry.CardinalDirection import CardinalDirection
from structures.graph import Vertex
from structures.problems.OrientedProblem import OrientedProblem
from structures.problems.VertexProblem import VertexProblem


class NeighbourAbsenceProblem(VertexProblem, OrientedProblem):
    def __init__(self, vertex: Vertex, direction: CardinalDirection):
        super().__init__(vertex)
        self.direction = direction

    @property
    def description(self) -> str:
        return f'No neighbours found on side {self.direction} of vertex {self.vertex.debug_string}.'