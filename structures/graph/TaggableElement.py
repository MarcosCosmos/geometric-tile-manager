from structures.graph.Tag import Tag


from abc import abstractmethod


class TaggableElement:
    """
    We can't use this class with Edge, which a namedtuple and therefore cannot be an element, but it already have a suitable repr
    :param type:
    :return:
    """

    @abstractmethod
    def generate_tag(self) -> Tag:
        ...

    def __str__(self) -> str:
        return str(self.generate_tag())

    def __repr__(self) -> str:
        return f'<{self.__class__.__module__}.{self.__class__.__name__} at {hex(id(self))}: {self.generate_tag()}>'