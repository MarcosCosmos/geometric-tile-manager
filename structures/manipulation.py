from typing import Final, MutableSequence, Sequence

from procedures.examination import are_aligned, covers_contents
from structures.graph import Box, Vertex, Edge, IndependantBox
from structures.geometry import Axis


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

class TileSegment(Segment):
    ...

class SegmentedBox(Segment):
    """
    A recursive structure for scaling subgraphs (contained within an AABB).

    Performs calculations and recursive segmentations to ensure integer tiles positions whilst also ensuring that margins are exact.
    """
    _segments: MutableSequence[Segment]
    segmentation_axis: Final[Axis]

    """
    A utility class created by passing in a Box.
    """
    def __init__(self, source: Box, scale_axis: Axis, segmentation_axis: Axis):
        super().__init__(source, scale_axis)
        self.segmentation_axis = segmentation_axis

        assert(covers_contents(source))
        self._segments = []
        forward_dir = segmentation_axis.directions[-1]
        perpendicular_forward_dir = segmentation_axis.perpendicular.directions[-1]

        (initial_edge, end_edge) = map(self.sides.__getitem__, segmentation_axis.directions)
        last_edge = initial_edge
        cur_negative, cur_positive = last_edge

        while last_edge.a is not end_edge.a:
            #creep along the sides until we find two aligned corners
            if are_aligned(cur_negative, cur_positive, segmentation_axis):
                cur_negative = cur_negative.neighbours[forward_dir]
                cur_positive = cur_negative.neighbours[forward_dir]
            elif cur_negative.location[segmentation_axis] < cur_positive.location[segmentation_axis]:
                cur_negative = cur_negative.neighbours[forward_dir]
            else:
                cur_positive = cur_negative.neighbours[forward_dir]

            #if we are aligned now, then try to creep inwards.
            if are_aligned(cur_negative, cur_positive, segmentation_axis):
                cur_perpendicular = cur_negative
                candidates: Sequence[Vertex] = cur_perpendicular.neighbours[perpendicular_forward_dir]
                are_connectable: bool = False
                while len(candidates) == 1 and not candidates[0].is_sentinel:
                    if candidates[0] is cur_negative:
                        are_connectable = True
                        break
                    else:
                        candidates = cur_perpendicular.neighbours[perpendicular_forward_dir]

                if are_connectable:
                    next_edge = Edge(cur_negative, cur_positive)
                    if next_edge.a.owner is last_edge.a.owner:
                        if next_edge.a is next_edge.b.owner:
                            #the segment is a single tile, no sub-segmentation is required.
                            self._segments.append(TileSegment(next_edge.a.owner, scale_axis))
                        else:
                            self._segments.append(SegmentedBox(IndependantBox(*{
                                each_dir.snake_case_name: each_vertex for (each_dir, each_vertex) in zip(
                                    (diag for card in segmentation_axis.directions for diag in card.diagonals),
                                    (*last_edge, *next_edge)
                                )
                            }), scale_axis, segmentation_axis.perpendicular))
                    last_edge = next_edge


