from structures.geomtry import OrientationEnum, CardinalDirection


from functools import cache


class Axis(OrientationEnum):
    @property
    @cache
    def perpendicular(self) -> 'Axis':
        return Axis(not self.value)

    @property
    @cache
    def directions(self) -> tuple[CardinalDirection, CardinalDirection]:
        return (CardinalDirection((self, False)), CardinalDirection((self, True)))

    HORIZONTAL = False
    VERTICAL = True