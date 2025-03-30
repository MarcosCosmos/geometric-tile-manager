from structures.geomtry.Axis import Axis
from structures.graph import Vertex
from structures.problems import AlignedProblem, VertexProblem


class VertexDistanceProblem(VertexProblem, AlignedProblem):
    """
    This can be applied to the sides of a two different Tiles or two different Tiles, as long as the sides are parallel and adjacent.
    """

    def __init__(self, target_vertex: Vertex, other_vertex: Vertex, orientation: Axis, expected_distance: int, actual_distance: int):
        super(VertexProblem).__init__(target_vertex)
        super(AlignedProblem, self).__init__(orientation)
        self.other_vertex = other_vertex
        self.expected_distance = expected_distance
        self.actual_distance = actual_distance

    @property
    def description(self) -> str:
        return f'{self.__class__.__name__}: The distance between vertices {self.vertex.debug_string} and {self.other_vertex.debug_string} should be limited to {self.expected_distance} units along the {self.axis} axis, but were found to be {self.actual_distance} units apart instead.'