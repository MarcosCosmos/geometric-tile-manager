from geometry.axis import Axis
from problems.state import StateProblem


class AlignedProblem(StateProblem):
    """
    A problem which is oriented along a particular axis.
    """
    axis: Axis
    def __init__(self, orientation: Axis):
        self.axis = orientation