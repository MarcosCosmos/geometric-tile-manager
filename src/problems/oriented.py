from geometry.direction import Direction
from problems.state import StateProblem


class OrientedProblem(StateProblem):
    direction: Direction
    def __init__(self, direction: Direction):
        self.direction = direction