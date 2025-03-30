import parse as ps
from structures.graph import TagMeta, _TagBase


from typing import ClassVar, Type, TypeVar


T = TypeVar('T')
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