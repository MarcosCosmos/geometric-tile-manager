from structures.geomtry.Direction import Direction
from structures.problems.StateProblem import StateProblem


class OrientedProblem(StateProblem):
    direction: Direction
    def __init__(self, direction: Direction):
        self.direction = direction