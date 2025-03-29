from structures.geomtry import AxisDataclass
from utility import enum_dataclass


import operator
from typing import Callable, TypeVar

T = TypeVar('T')

@enum_dataclass(frozen=True)
class Vector(AxisDataclass[T]):

    def __str__(self):
        return f'Vector({self.horizontal},{self.vertical})'

    @property
    def debug_string(self) -> str:
        return f'({self.horizontal},{self.vertical})'

    def __op(self, other: 'Vector'[T], op: Callable[['Vector'[T], 'Vector'[T]], 'Vector'[T]]):
        return Vector(*tuple(op(*each) for each in zip(self, other)))

    def __add__(self, other: 'Vector'[T]):
        return self.__op(other, operator.add)

    def __sub__(self, other: 'Vector'[T]) -> 'Vector'[T]:
        return self.__op(other, operator.sub)

    def __matmul__(self, other: 'Vector'[T]) -> 'Vector'[T]:
        return self.__op(other, operator.mul)

    def __mul__(self, scale: T) -> 'Vector'[T]:
        return Vector(*tuple(each * scale for each in self))