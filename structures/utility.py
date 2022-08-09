from __future__ import annotations

import dataclasses
from enum import Enum
from functools import singledispatchmethod
from types import GenericAlias
from typing import TypeVar, Type, get_args, get_origin, Union, Any, Callable, Generic, Final, no_type_check_decorator
T = TypeVar('T')
KT = TypeVar('KT')
VT = TypeVar('VT')

class DefaultedDict(dict[KT, VT]):
    """
    Intended to initially provide default values from the configuration, but allow them to be overridden on a per-tile basis.
    Slightly different to dict.setdefault, which would need to be set per-tile.
    """
    _fallback_fn: Callable[[], VT]
    def __init__(self, fallback_fn, seq=None, **kwargs):
        super().__init__(seq=seq,**kwargs)
        self._fallback_fn = fallback_fn

    def __get_item__(self, key) -> VT:
        try:
            return super().__getitem__(key)
        except KeyError:
            return self._fallback_fn()


class DataclassEnum(Enum):
    """
    provides a snake_case_name member string for each enum member, which is more suitable for member names in corresponding dataclasses.
    """

    snake_case_name: Final[str]

    def __init__(self, *args):
        self.snake_case_name = self.name.lower()
    # _INDEX: Final[int]

    # def __new__(cls, *args):
    #     if len(cls.__bases__) >= 2:
    #         value_type = cls.__bases__[-2]
    #         obj = value_type.__new__(cls, *args)
    #         obj._value_ = value_type.__new__(value_type, *args)
    #     else:
    #         obj = object.__new__(cls)
    #         obj._value_ = args[0]
    #         if len(args) > 1:
    #             obj._value_ = args
    #         else:
    #             obj._value_ = args[0]
    #
    #     obj._INDEX = len(cls.__members__)
    #     return obj

    # def __index__(self):
    #     return  self._INDEX

    # def __int__(self):
    #     return self._INDEX


DCEnumT = TypeVar('DCEnumT', bound=DataclassEnum)

#todo: possibly look into leveraging the performance of slots whilst still allowing enum-keyed lookup. E.g. with a tuple we can store an index on the enum member, unsure how to leverage that in a dataclass at this stage.

class EnumDataclassBase(Generic[DCEnumT, T]):
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

    def _replace(instance, data: dict[DCEnumT, VT]) -> EnumDataclassBase[DCEnumT]:
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

# class EnumTuple(Generic[GTMEnumT, VT]):
#     """
#     An immutable tuple obj.
#
#     These are similar to NanedTuple except that it is keyed by an enum instead of by index or by member number.
#
#     Technically, in practice, it implemented by wrapping a NamedTuple and can be accessed all three ways.
#
#     However, type hints are only provided for enum-based usage for now.
#     """
#     @overload
#     @classmethod
#     def _make(cls, data: dict[GTMEnumT, VT]) -> EnumTuple[GTMEnumT, VT]:
#        ...
#
#     @overload
#     @classmethod
#     def _make(cls, data: Iterable) -> EnumTuple[GTMEnumT, VT]:
#        ...
#
#     @classmethod
#     def _make(cls, data) -> EnumTuple[GTMEnumT, VT]:
#         ...
#
#
#     @singledispatchmethod
#     @staticmethod
#     def _replace(cls, *args, **kwargs) -> EnumTuple:
#         ...
#
#     @_replace.register(dict)
#     @staticmethod
#     def _(data: dict[KT, VT]):
#         ...
#
#     def _asdict(self) -> dict[KT, VT]:
#         ...
#
#     # @overload
#     # def __getitem__(self, item: enum_type) -> T:
#     #     ...
#     #
#     # @overload
#     # def __getitem__(self, item: int) -> T:
#     #     ...
#     #
#     def __getitem__(self, item: KT) -> VT:
#        ...

# def enum_tuple(base=None, /, *, enum_type: Type[DCEnumT], value_type: Type[VT]):
#     """
#         Small wrapper around a NamedTuple that allows us to construct the tuple from a dict keyed on DirectionalEnums, via _make and _replace. Also modifies _asdict to return the enum as keys instead.
#         Currently only supports a tuple of a single type (may extend later)
#     """
#     # base = collections.namedtuple('EnumTuple', [each.snake_case_name for each in enum_type])
#     def wrap(base):
#         class _EnumTuple(base):
#             @singledispatchmethod
#             @classmethod
#             def _make(cls, data):
#                 return cls(*data)
#
#             @_make.register(dict)
#             @classmethod
#             def _(cls, data: dict[enum_type, value_type]):
#                 return cls(*map(data.__getitem__, enum_type))
#
#             @singledispatchmethod
#             @classmethod
#             def _replace(cls, *args, **kwargs) -> _EnumTuple:
#                 return base._replace(cls, **kwargs)
#
#
#             @_replace.register(dict)
#             @classmethod
#             def _(data: dict[enum_type, value_type]):
#                 return base._replace(**{key.snake_case_name: value for (key, value) in data.items()})
#
#             def _asdict(self):
#                 return {enum_type[key]: value for (key, value) in base._asdict()}
#
#             def __getitem__(self, item: DCEnumT) -> VT:
#                 return base.__getitem__(self, item)
#         return _EnumTuple
#
#     # See if we're being called as @dataclass or @dataclass().
#     if base is None:
#         # We're called with parens.
#         return wrap
#
#     return wrap(base)

@no_type_check_decorator
def enum_dataclass(cls=None, /, **kwargs):
    """
    Companion decorator for turning subclasses of EnumDataclassBase into actual dataclasses, allowing them to separately specific other parameters like frozen.
    Similar to a dataclass, except that the fields are deduced from the type parameters of EnumDataclassBase instead of from annotations (annotations may be provided for autocomplete/DE hinting but have no effect on the runtime dataclass.)

    :param cls: the class to decorate.
    :param enum: the enum whose members act as dataclass fields. These are not inferred from the inherited classes of the decorated class at this time, although they should match to ensure proper hinting.
    :param value: the value type (can be a real type or generic alias)
    :param kwargs: these arguments are forwarded to the call to dataclasses.dataclass
    :return:
    """
    def wrap(cls):
        enum: Type[DCEnumT]
        value: Type[T]
        enum, value = resolve_type_arguments(EnumDataclassBase, cls)
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

def resolve_type_arguments(query_type: Type, target_type: Union[Type, GenericAlias]) -> tuple[Union[Type, TypeVar], ...]:
    """
    Resolves the type arguments of the query type as supplied by the target type of any of its bases.
    Operates in a tail-recursive fashion, and drills through the hierarchy of generic base types breadth-first in left-to-right order to correctly identify the type arguments that need to be supplied to the next recursive call.

    raises a TypeError if they target type was not an instance of the query type.

    :param query_type: Must be supplied without args (e.g. Mapping not Mapping[KT,VT]
    :param target_type: Must be supplied with args (e.g. Mapping[KT, T] or Mapping[str, int] not Mapping)
    :return: A tuple of the arguments given via target_type for the type parameters of for the query_type, if it has any parameters, otherwise an empty tuple. These arguments may themselves be TypeVars.
    """
    target_origin = get_origin(target_type)
    if target_origin is None:
        if target_type is query_type:
            return target_type.__parameters__
        else:
            target_origin = target_type
            supplied_args = None
    else:
        supplied_args = get_args(target_type)
        if target_origin is query_type:
            return supplied_args
    param_set = set()
    param_list = []
    for (i, each_base) in enumerate(target_origin.__orig_bases__):
        each_origin = get_origin(each_base)
        if each_origin is not None:
            # each base is of the form class[T], which is a private type _GenericAlias, but it is formally documented to have __parameters__
            for each_param in each_base.__parameters__:
                if each_param not in param_set:
                    param_set.add(each_param)
                    param_list.append(each_param)
            if issubclass(each_origin, query_type):
                if supplied_args is not None and len(supplied_args) > 0:
                    params_to_args = {key: value for (key, value) in zip(param_list, supplied_args)}
                    resolved_args = tuple(params_to_args[each] for each in each_base.__parameters__)
                    return resolve_type_arguments(query_type, each_base[resolved_args]) #each_base[args] fowards the args to each_base, it is not quite equivalent to GenericAlias(each_origin, resolved_args)
                else:
                    return resolve_type_arguments(query_type, each_base)
        elif issubclass(each_base, query_type):
            return resolve_type_arguments(query_type, each_base)
    if not issubclass(target_origin, query_type):
        raise ValueError(f'{target_type} is not a subclass of {query_type}')
    else:
        return ()