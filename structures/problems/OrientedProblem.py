from structures.geomtry import Direction
from structures.problems import StateProblem


class OrientedProblem(StateProblem):
    direction: Direction
    def __init__(self, direction: Direction):
        self.direction = direction