#todo: possibly look into leveraging the performance of slots whilst still allowing enum-keyed lookup. E.g. with a tuple we can store an index on the enum member, unsure how to leverage that in a dataclass at this stage.

from structures.utility import DataEnum

import dataclasses
from functools import singledispatchmethod
from typing import Generic, TypeVar

T = TypeVar('T')
DCEnumT = TypeVar('DCEnumT', bound=DataEnum)

class EnumDataclass(Generic[DCEnumT, T]):
    @singledispatchmethod
    @classmethod
    def _make(cls, *args, **kwargs):
        return cls(*args)
    @_make.register(dict)
    @classmethod
    def _make(cls, data: dict[DCEnumT, T]):
        return cls(**{
            key.snake_case_name: value
            for (key, value) in data.items()
        })

    def _replace(instance, data: dict[DCEnumT, VT]) -> EnumDataclass[DCEnumT]:
        """
        Like dataclasses.replace, but keyed using enum members.
        Effectively sugar that calls dataclasses.replace with each key swapped for key.snake_case_name to allow them to be used as **kwargs without the boilerplate of accessing key.snake_case_name each time.
        :param data:
        :return:
        """
        return dataclasses.replace(instance, **{
            key.snake_case_name: value
            for (key, value) in data
        })

    def _as_dict(self) -> dict[DCEnumT, VT]:
        return {each: self[each] for each in DCEnumT}

    def __getitem__(self, item: DCEnumT) -> VT:
        return getattr(self, item.snake_case_name)

    def __setitem__(self, item: DCEnumT, value: VT):
        return setattr(self, item.snake_case_name, value)

    def __iter__(self):
        return iter(map(lambda x: getattr(self, x.name), dataclasses.fields(self)))
    
@no_type_check_decorator
def enum_dataclass(cls=None, /, **kwargs):
    """
    Companion decorator for turning subclasses of EnumDataclass into actual dataclasses, allowing them to separately specific other parameters like frozen.
    Similar to a dataclass, except that the fields are deduced from the type parameters of EnumDataclass instead of from annotations (annotations may be provided for autocomplete/DE hinting but have no effect on the runtime dataclass.)

    :param cls: the class to decorate.
    :param enum: the enum whose members act as dataclass fields. These are not inferred from the inherited classes of the decorated class at this time, although they should match to ensure proper hinting.
    :param value: the value type (can be a real type or generic alias)
    :param kwargs: these arguments are forwarded to the call to dataclasses.dataclass
    :return:
    """
    def wrap(cls):
        enum: Type[DCEnumT]
        value: Type[T]
        enum, value = resolve_type_arguments(EnumDataclass, cls)
        new_annotations = {key.snake_case_name: value for key in enum}
        try:
            original_annotations = cls.__annotations__
        except AttributeError:
            original_annotations = {}
        cls.__annotations__ = {key.snake_case_name: value for key in enum}
        result = dataclasses.dataclass(cls, **kwargs)
        result.__annotations__ = {**original_annotations, **new_annotations}

        return result
    # See if we're being called as @dataclass or @dataclass().
    if cls is None:
        # We're called with parens.
        return wrap

    # We're called as @dataclass without parens.
    return wrap(cls)