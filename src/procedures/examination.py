from abc import abstractmethod
from functools import singledispatch
from typing import Sequence

from geometry.axis import Axis
from geometry.direction.cardinal import CardinalDirection
from geometry.direction.diagonal import DiagonalDirection
from geometry.graph.box import Box
from geometry.graph.edge import Edge
from geometry.graph.tile import Tile
from geometry.graph.vertex import Vertex
from geometry.graph.window import Window
from problems.edge import NotAxisAlignedProblem, BrokenEdgeProblem, OutOfOrderProblem
from problems.state import StateProblem
from problems.vertex import NeighbourAbsenceProblem


def is_divided(target: Tile) -> bool:
    for each_side in CardinalDirection:
        each_edge = target.sides[each_side]

        (forward_neighbours, backward_neighbours) = (
            each_edge.a.neighbours[each_side.axis.perpendicular.directions[-1]],
            each_edge.b.neighbours[each_side.axis.perpendicular.directions[0]]
        )

        if (
                len(forward_neighbours) != 1
                or
                forward_neighbours[0] is not each_edge.b
                or
                len(backward_neighbours) != 1
                or
                backward_neighbours[0] is not each_edge.a
        ):
            return True

    return False

def are_aligned(a: Vertex, b: Vertex, alignment_axis: Axis) -> bool:
    """
    Determines whether or not two vertices are aligned to the same value on the given axis.
    This provides shorthand/readability since accessing locations makes the syntax verbose.
    :param a:
    :param b:
    :param alignment_axis:
    :return:
    """
    return a.location[alignment_axis] == b.location[alignment_axis]

def covers_contents(target: Box):
    """
    Determines whether or not the box strictly encloses all tiles it intersects with.

    First tests that the box is a true AABB via validate(Box), then tests that each of the corners have the same role in this box as in their owner tiles.
    :param target:
    :return:
    """

    if len(validate(target)) > 0:
        return False
    else:
        for each_diagonal in DiagonalDirection:
            each_corner = target.corners[each_diagonal]
            if each_corner.role is not each_diagonal:
                return False
    return True


@singledispatch
@abstractmethod
def validate(target) -> Sequence[StateProblem]:
    """
    Inspects a target graph or graph-utility object to identify possible constraint violations. E.g. an Edge is invalid if the locations its Vertices differ along more than one Axis or if the first Vertex is positioned after the second.
    Raises a TypeError when passed types it is not implemented for.
    :param target:
    :return: A potentially-empty sequence of StateProblem objects describing any violated constraints.
    """

@validate.register
def _(target: Edge) -> Sequence[StateProblem]:
    """
    Valid edges must be aligned (parallel) to an axis, must start before they end (or at the same place), and the start and end must be reachable through an unambigous path of vertices that lie strictly on the edge
    - Note: the latter condition can observed by ensuring that there each vertex has only one neighbor in both directions along that line.
    -- Having exactly one neighbour in a direction implies being the only neighbour in the reverse direction, so one directional tests suffice.
    :param target:
    :return:
    """
    result = []

    forward_direction = target.axis.directions[-1]

    if not (target.a.location.horizontal == target.b.location.horizontal or target.a.location.vertical == target.a.location.vertical):
        result.append(NotAxisAlignedProblem(target))

    if target.a.location.horizontal > target.b.location.horizontal:
        result.append(OutOfOrderProblem(target, Axis.HORIZONTAL))

    if target.a.location.vertical > target.b.location.vertical:
        result.append(OutOfOrderProblem(target, Axis.VERTICAL))

    if len(result) == 0:
        current = target.a
        while current is not target.b:
            options = current.neighbours[forward_direction]
            if len(options) != 1:
                result.append(BrokenEdgeProblem(target))
                break


    return result


@validate.register
def _(target: Box) -> Sequence[StateProblem]:
    result = []

    for each_edge in target.sides:
        result += validate(each_edge)

    return result


@validate.register
def _(target: Window) -> Sequence[StateProblem]:
    result = list(validate.registry[Box](target))

    for each_side in CardinalDirection:
        each_side: CardinalDirection
        each_edge = target.sides[each_side]
        (forward_neighbours, backward_neighbours) = [each_edge.a.neighbours[dir] for dir in each_side.axis.directions]
        if (
                len(forward_neighbours) != 1
                or
                forward_neighbours[0] is not each_side.b
                or
                len(backward_neighbours) != 1
                or
                backward_neighbours[0] is not each_side.a
        ):
            result.append(BrokenEdgeProblem(each_edge))

    for each_corner in target.corners:
        for each_side in CardinalDirection:
            if len(each_corner.neighbours[each_side]) == 0:
                result.append(NeighbourAbsenceProblem(each_corner, each_side))

    return result

