from types import GenericAlias
from typing import TypeVar, Generic

from utility.resolve_type_arguments import resolve_type_arguments


class MyTestCase:
    def test_something(self):

        #todo: convert this into actual unit tests with asserts instead of printouts, split up cases.
        
        T = TypeVar('T')
        U = TypeVar('U')
        Q = TypeVar('Q')
        R = TypeVar('R')

        W = TypeVar('W')
        X = TypeVar('X')
        Y = TypeVar('Y')
        Z = TypeVar('Z')

        class A(Generic[T, U, Q, R]):
            ...
        class NestedA(Generic[T, U, Q]):
            ...
        class NestedB(Generic[T]):
            ...
        class NoParams:
            ...
        class B(NoParams, NestedA[U, Q, U], A[int, NestedA[Q, Q, Q], Q, U], NestedB[R]):
            ...
        class C(B[T, str, int]):
            ...
        class D(C[int]):
            ...
        class E(D):
            ...
        class F(E):
            ...
        class G(Generic[T]):
            ...
        class H(Generic[T]):
            ...
        class I(G[int]):
            ...
        class J(I, H[str]):
            ...

        my_types = (A, B, C, D, E, F, G, H, I, J)
        fill_vars = [W, X, Y, Z]

        max_len = 0
        all_commands: list[str] = []
        all_results = []
        for i in range(len(my_types)):
            for k in range(i + 1):
                commands: list[str]
                results = []
                if len(my_types[i].__parameters__):
                    generic = GenericAlias(my_types[i], my_types[i].__parameters__)
                    filled = GenericAlias(my_types[i],
                                          tuple(fill_vars[i] for i in range(len(my_types[i].__parameters__))))
                    commands = [
                        f'resolve_type_arguments({str(my_types[k].__name__)}, {my_types[i].__name__})',
                        f'resolve_type_arguments({str(my_types[k].__name__)}, {my_types[i].__name__}[{",".join(fill_vars[i].__name__ for i in range(len(my_types[i].__parameters__)))}])'
                    ]
                    results.append(resolve_type_arguments(my_types[k], generic))
                    results.append(resolve_type_arguments(my_types[k], filled))
                else:
                    commands = [
                        f'resolve_type_arguments({str(my_types[k].__name__)}, {my_types[i].__name__})'
                    ]
                    results.append(resolve_type_arguments(my_types[k], my_types[i]))
                for command in commands:
                    max_len = max(max_len, len(command))
                all_commands += commands
                all_results += results

        max_len += 7
        for (command, result) in zip(all_commands, all_results):
            actual_len = 7 + len(command)
            padding = max_len + 5 - actual_len
            print(f'print({command}) {" " * padding} # {result}')


if __name__ == '__main__':
    MyTestCase().test_something()
