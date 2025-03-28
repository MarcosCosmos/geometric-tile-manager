from typing import Optional, Sequence

from procedures.examination import validate, are_aligned
from structures.geomtry.geometry import CardinalDirection
from structures.graph import Edge, Vertex

"""
Procedures which determine actionable regions for the purpose of manipulations.
Not to be confused with the navigation module/file, which is for more basic activities like window focus.
"""

def find_parallel_end(target: Edge, direction: CardinalDirection) -> Optional[Edge]:
    """
    Computes the far end edge parallel to the target edge in the target direction, or None if there is no further edge.
    :param target:
    :param direction:
    :param new_size:
    :return:
    """
    initial_a, initial_b = target

    #first build stacks that show the furthest reachable nodes from the ends of the edge, along the straight lines perpendicular to the edge.
    stack_a: list[Vertex] = [initial_a]
    stack_b: list[Vertex] = [initial_b]

    for each_stack in (stack_a, stack_b):
        next: Sequence[Vertex] = each_stack[-1].neighbours[direction]
        while len(next) == 1 and not next[0].is_sentinel:
            each_stack.append(next[0])
            next = next[0].neighbours[direction]

    # now work backwards through the stacks finding the furthest pair we can draw an edge for.
    while all(len(each) > 2 for each in (stack_a, stack_b)):
        vertex_a = stack_a[-1]
        vertex_b = stack_b[-1]
        if are_aligned(vertex_a, vertex_b, direction.axis.perpendicular):
            candidate = Edge(vertex_a, vertex_b)
            if validate(candidate):
                return candidate
            else:
                stack_a.pop()
                stack_b.pop()
        else:
            if direction.is_positive == vertex_a.location[direction.axis] > vertex_b.location[direction.axis]:
                stack_a.pop()
            else:
                stack_b.pop()

    return None #no viable result was found.