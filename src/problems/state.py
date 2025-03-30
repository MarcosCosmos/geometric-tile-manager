from abc import ABC, abstractmethod


class StateProblem(ABC):
    """
    The description should be
    """
    @property
    @abstractmethod
    def description(self) -> str:
        ...