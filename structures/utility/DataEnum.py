from enum import Enum
from typing import Final


class DataEnum(Enum):
    """
    provides a snake_case_name member string for each enum member, which is more suitable for member names in corresponding dataclasses.
    """

    snake_case_name: Final[str]

    def __init__(self, *args):
        self.snake_case_name = self.name.lower()