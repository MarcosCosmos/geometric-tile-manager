from structures.problems.BoxProblem import BoxProblem


class BoxTooLargeForConstraintsProblem(BoxProblem):
    @property
    def description(self) -> str:
        return f'The box {self.box.debug_string} was too small to achieve all desired max segment sizes.'