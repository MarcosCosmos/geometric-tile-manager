from structures.geomtry.Axis import Axis
from structures.problems.StateProblem import StateProblem


class AlignedProblem(StateProblem):
    """
    A problem which is oriented along a particular axis.
    """
    axis: Axis
    def __init__(self, orientation: Axis):
        self.axis = orientation