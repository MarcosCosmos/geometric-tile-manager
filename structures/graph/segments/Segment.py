from typing import Final

from procedures.examination import covers_contents
from structures.geomtry import Axis
from structures.graph import Box


class Segment:
    """
    A component of a SegmentedBox.
    The scale axis is the axis along which size is computed.
    """
    source: Final[Box]
    scale_axis: Final[Axis]

    def __init__(self, source: Box, scale_axis: Axis):
        assert(covers_contents(source))
        self.source = source
        self.scale_axis = scale_axis
