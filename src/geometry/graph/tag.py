from abc import abstractmethod, ABCMeta
from types import GenericAlias
from typing import ClassVar, Type, TypeVar, Generic

import parse as ps

from utility.helpers import resolve_type_arguments

T = TypeVar('T')


class _TagBase(Generic[T]):
    """
    Exists purely to allow Tag to inherit something other than itself for the purpose of Element type resolution.
    """
    ...


class TagMeta(ABCMeta):
    def __new__(mcls, name, bases, namespace, **kwargs):
        # get_args
        cls = super().__new__(mcls, name, bases, namespace, **kwargs)
        (cls.Element,) = resolve_type_arguments(_TagBase, GenericAlias(cls, cls.__parameters__))
        cls.__annotations__

        return cls


class Tag(_TagBase[T], metaclass=TagMeta):
    """
    Tags help external users to refer to graph elements such as Tiles, Vertices, and Edges e.g. via IPC.
    They are composite objects describing the element's type and role.
    -- Tiles are the principle basis for uniqueness. They receive numerical ids. In future, custom string strings might be allowed as ids.
    --- Note: other properties like window title etc are not part of identification here, that is left to other libraries to manage in combination with what is provided here.
    -- Vertices are identified by their owning Tile and the corner they correspond to.
    -- Other items, like general Boxes and Edges are defined by the vertices that bound them.
    - Although not all elements are actionable, it is generally valid for a user to specify an arbitrary element as a search term to find a suitable element for a given action, which is generally the smallest containing element of the correct type.

    They can be converted to strings and will be parseable in future.


    todo: equality, hash, decode, etc.
    """

    Element: ClassVar[Type[T]]
    format: ClassVar[str]

    def __str__(self):
        return f'{self.Element.__name__}({self.inner_str})'

    @property
    def inner_str(self) -> str:
        return self.format.format(self=self)

    def __repr__(self):
        return str(self)

    @staticmethod
    def parse(text: str) -> 'Tag':
        (target_name, inner_text) = ps.parse('{}({})', text).fixed
        for each_cls in Tag.__subclasses__():
            if target_name == each_cls.Element.__name__:
                return each_cls.parse(inner_text)
        raise "Could not determine tag type"


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
