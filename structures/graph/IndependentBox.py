from structures.graph import Box


class IndependentBox(Box):
    """
    Base class for non-tile Boxes. Required to be a valid box, but can cover one or more other boxes.

    These boxes are generally temporary and are primarily used for to store computational results as part of manipulation processes
    """