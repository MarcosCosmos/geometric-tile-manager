from structures.graph import Vertex
from structures.problems import StateProblem


class VertexProblem(StateProblem):
    vertex: Vertex
    def __init__(self, vertex: Vertex):
        self.vertex = vertex