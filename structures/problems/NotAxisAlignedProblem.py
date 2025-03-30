from structures.problems.EdgeProblem import EdgeProblem


class NotAxisAlignedProblem(EdgeProblem):
    @property
    def description(self) -> str:
        return f'{self.edge.debug_string} is aligned to either axis.'