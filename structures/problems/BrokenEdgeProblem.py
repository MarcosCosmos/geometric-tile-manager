from structures.problems import EdgeProblem


class BrokenEdgeProblem(EdgeProblem):
    @property
    def description(self) -> str:
        return f'The vertices along edge {self.edge.debug_string} must be the only neighbours in the directions facing one another, but they are not.'