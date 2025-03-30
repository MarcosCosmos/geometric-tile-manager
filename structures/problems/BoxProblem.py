from structures.graph import Box
from structures.problems.StateProblem import StateProblem


class BoxProblem(StateProblem):
    box: Box
    def __init__(self, box: Box):
        self.box = box