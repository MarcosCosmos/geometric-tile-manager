from structures.graph._TagBase import _TagBase


from abc import ABCMeta


class TagMeta(ABCMeta):
    def __new__(mcls, name, bases, namespace, **kwargs):
        # get_args
        cls = super().__new__(mcls, name, bases, namespace, **kwargs)
        (cls.Element,) = resolve_type_arguments(_TagBase, GenericAlias(cls, cls.__parameters__))
        cls.__annotations__

        return cls