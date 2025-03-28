# class EnumTuple(Generic[GTMEnumT, VT]):
#     """
#     An immutable tuple obj.

#     These are similar to NanedTuple except that it is keyed by an enum instead of by index or by member number.

#     Technically, in practice, it implemented by wrapping a NamedTuple and can be accessed all three ways.

#     However, type hints are only provided for enum-based usage for now.
#     """
#     @overload
#     @classmethod
#     def _make(cls, data: dict[GTMEnumT, VT]) -> EnumTuple[GTMEnumT, VT]:
#        ...

#     @overload
#     @classmethod
#     def _make(cls, data: Iterable) -> EnumTuple[GTMEnumT, VT]:
#        ...

#     @classmethod
#     def _make(cls, data) -> EnumTuple[GTMEnumT, VT]:
#         ...


#     @singledispatchmethod
#     @staticmethod
#     def _replace(cls, *args, **kwargs) -> EnumTuple:
#         ...

#     @_replace.register(dict)
#     @staticmethod
#     def _(data: dict[KT, VT]):
#         ...

#     def _asdict(self) -> dict[KT, VT]:
#         ...

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

#             @_make.register(dict)
#             @classmethod
#             def _(cls, data: dict[enum_type, value_type]):
#                 return cls(*map(data.__getitem__, enum_type))

#             @singledispatchmethod
#             @classmethod
#             def _replace(cls, *args, **kwargs) -> _EnumTuple:
#                 return base._replace(cls, **kwargs)


#             @_replace.register(dict)
#             @classmethod
#             def _(data: dict[enum_type, value_type]):
#                 return base._replace(**{key.snake_case_name: value for (key, value) in data.items()})

#             def _asdict(self):
#                 return {enum_type[key]: value for (key, value) in base._asdict()}

#             def __getitem__(self, item: DCEnumT) -> VT:
#                 return base.__getitem__(self, item)
#         return _EnumTuple

#     # See if we're being called as @dataclass or @dataclass().
#     if base is None:
#         # We're called with parens.
#         return wrap

#     return wrap(base)