from structures.problems.BoxProblem import BoxProblem


class BoxTooSmallForMarginsProblem(BoxProblem):
    @property
    def description(self) -> str:
        return f'The box {self.box} was too small to allocate margins.'