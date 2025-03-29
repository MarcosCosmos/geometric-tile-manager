from types import GenericAlias
from typing import Union, Type, TypeVar, get_args, get_origin


def resolve_type_arguments(query_type: Type, target_type: Type | GenericAlias) -> tuple[Type | TypeVar, ...]:
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