from __future__ import annotations

from structures.settings import StaticConfiguration

class Settings:
    static_config: StaticConfiguration
    # active_constraints: ConstraintManager

    def __init__(self):
        self.static_config = StaticConfiguration()
