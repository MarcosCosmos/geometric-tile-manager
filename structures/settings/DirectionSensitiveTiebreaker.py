class DirectionSensitiveTiebreaker:
    EXTREME_SYMMETRICAL_NORTH_WEST_AND_SOUTH_EAST = lambda direction, options: options[0 if direction.is_positive else -1]