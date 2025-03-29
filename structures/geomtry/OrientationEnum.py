from utility import DataEnum

class OrientationEnum(DataEnum):
    """
    Base class for utility enums relating to directions or axes.

     __str__ and __repr__ representations are simplified, since this is only intended for unique enums and this makes for easier debugging for the most part.
    """

    def __str__(self) -> str:
        return self.snake_case_name

    def __repr__(self):
        return f'<{self.__class__.__name__}.{self.name}>'