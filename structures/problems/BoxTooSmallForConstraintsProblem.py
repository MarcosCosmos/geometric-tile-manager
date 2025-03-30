from structures.problems import BoxProblem


class BoxTooSmallForConstraintsProblem(BoxProblem):
    @property
    def description(self) -> str:
        return f'The box {self.box.debug_string} was too small to achieve all desired minimum segment sizes.'