from abc import abstractmethod, ABC

from structures.graph import Box, Edge, Tile, Window, Vertex

from structures.geometry import Axis, CardinalDirection, Direction


class StateProblem(ABC):
    """
    The description should be
    """
    @property
    @abstractmethod
    def description(self) -> str:
        ...

class IllegalProblem(StateProblem):
    ...

class VertexProblem(StateProblem):
    vertex: Vertex
    def __init__(self, vertex: Vertex):
        self.vertex = vertex

class EdgeProblem(StateProblem):
    edge: Edge
    def __init__(self, edge: Edge):
        self.edge = edge

class BoxProblem(StateProblem):
    box: Box
    def __init__(self, box: Box):
        self.box = box
class TileProblem(BoxProblem):
    box: Tile

    def __init__(self, box: Tile):
        super().__init__(box)

    @property
    def tile(self) -> Tile:
        return self.box
class WindowProblem(TileProblem):
    box: Window
    def __init__(self, box: Window):
        super().__init__(box)
    @property
    def window(self) -> Window:
        return self.box

class AlignedProblem(StateProblem):
    """
    A problem which is oriented along a particular axis.
    """
    axis: Axis
    def __init__(self, orientation: Axis):
        self.axis = orientation

class OrientedProblem(StateProblem):
    direction: Direction
    def __init__(self, direction: Direction):
        self.direction = direction


class OutOfOrderProblem(EdgeProblem):
    def __init__(self, edge, axis):
        super().__init__(edge)
        self.axis = axis
    @property
    def description(self) -> str:
        return f'The vertices of {self.edge.debug_string} are out of order along the {self.axis} axis.'

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

class InsufficientDistanceProblem(VertexDistanceProblem):
    ...

class ExcessiveDistanceProblem(VertexDistanceProblem):
    ...

class NotAxisAlignedProblem(EdgeProblem):
    @property
    def description(self) -> str:
        return f'{self.edge.debug_string} is aligned to either axis.'

class BrokenEdgeProblem(EdgeProblem):
    @property
    def description(self) -> str:
        return f'The vertices along edge {self.edge.debug_string} must be the only neighbours in the directions facing one another, but they are not.'

class NeighbourAbsenceProblem(VertexProblem, OrientedProblem):
    def __init__(self, vertex: Vertex, direction: CardinalDirection):
        super().__init__(vertex)
        self.direction = direction

    @property
    def description(self) -> str:
        return f'No neighbours found on side {self.direction} of vertex {self.vertex.debug_string}.'

class BoxTooSmallForMarginsProblem(BoxProblem):
    @property
    def description(self) -> str:
        return f'The box {self.box} was too small to allocate margins.'


class BoxTooSmallForConstraintsProblem(BoxProblem):
    @property
    def description(self) -> str:
        return f'The box {self.box.debug_string} was too small to achieve all desired minimum segment sizes.'

class BoxTooLargeForConstraintsProblem(BoxProblem):
    @property
    def description(self) -> str:
        return f'The box {self.box.debug_string} was too small to achieve all desired max segment sizes.'

