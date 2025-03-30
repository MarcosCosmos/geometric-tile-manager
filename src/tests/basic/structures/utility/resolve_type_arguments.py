from types import GenericAlias
from typing import TypeVar, Generic

from utility.helpers import resolve_type_arguments


class MyTestCase:
    def test_from_reddit(self):
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

        print(resolve_type_arguments(A, A))  # (~T, ~U, ~Q, ~R)
        print(resolve_type_arguments(A, A[W, X, Y, Z]))  # (~W, ~X, ~Y, ~Z)
        print(resolve_type_arguments(A, B))  # (<class 'int'>, __main__.NestedA[~Q, ~Q, ~Q], ~Q, ~U)
        print(resolve_type_arguments(A, B[W, X, Y]))  # (<class 'int'>, __main__.NestedA[~X, ~X, ~X], ~X, ~W)
        print(resolve_type_arguments(B, B))  # (~U, ~Q, ~R)
        print(resolve_type_arguments(B, B[W, X, Y]))  # (~W, ~X, ~Y)
        print(resolve_type_arguments(A, C))  # (<class 'int'>, __main__.NestedA[str, str, str], <class 'str'>, ~T)
        print(resolve_type_arguments(A, C[W]))  # (<class 'int'>, __main__.NestedA[str, str, str], <class 'str'>, ~W)
        print(resolve_type_arguments(B, C))  # (~T, <class 'str'>, <class 'int'>)
        print(resolve_type_arguments(B, C[W]))  # (~W, <class 'str'>, <class 'int'>)
        print(resolve_type_arguments(C, C))  # (~T,)
        print(resolve_type_arguments(C, C[W]))  # (~W,)
        print(resolve_type_arguments(A,
                                     D))  # (<class 'int'>, __main__.NestedA[str, str, str], <class 'str'>, <class 'int'>)
        print(resolve_type_arguments(B, D))  # (<class 'int'>, <class 'str'>, <class 'int'>)
        print(resolve_type_arguments(C, D))  # (<class 'int'>,)
        print(resolve_type_arguments(D, D))  # ()
        print(resolve_type_arguments(A,
                                     E))  # (<class 'int'>, __main__.NestedA[str, str, str], <class 'str'>, <class 'int'>)
        print(resolve_type_arguments(B, E))  # (<class 'int'>, <class 'str'>, <class 'int'>)
        print(resolve_type_arguments(C, E))  # (<class 'int'>,)
        print(resolve_type_arguments(D, E))  # ()
        print(resolve_type_arguments(E, E))  # ()
        print(resolve_type_arguments(A,
                                     F))  # (<class 'int'>, __main__.NestedA[str, str, str], <class 'str'>, <class 'int'>)
        print(resolve_type_arguments(B, F))  # (<class 'int'>, <class 'str'>, <class 'int'>)
        print(resolve_type_arguments(C, F))  # (<class 'int'>,)
        print(resolve_type_arguments(D, F))  # ()
        print(resolve_type_arguments(E, F))  # ()
        print(resolve_type_arguments(F, F))  # ()
        print(resolve_type_arguments(G, J))


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
    MyTestCase().test_from_reddit()
