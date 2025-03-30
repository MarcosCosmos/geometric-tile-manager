from structures.problems import EdgeProblem


class OutOfOrderProblem(EdgeProblem):
    def __init__(self, edge, axis):
        super().__init__(edge)
        self.axis = axis
    @property
    def description(self) -> str:
        return f'The vertices of {self.edge.debug_string} are out of order along the {self.axis} axis.'