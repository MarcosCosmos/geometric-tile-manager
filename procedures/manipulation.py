from typing import Sequence, Optional

from procedures.examination import is_divided, are_aligned, covers_contents, validate
from structures.geomtry.Axis import Axis
from structures.geomtry.Vector import Vector
from structures.graph import Wall, Window, Edge, Vertex, Box
from structures.manager import GeometricTileManager
from structures.problems import BoxTooSmallForMarginsProblem, StateProblem
from structures.geomtry.CardinalDirection import CardinalDirection

def establish_connections_along_injection_axis(target: Window, parallel_axis: Axis, neighbours_negative: tuple[Sequence[Vertex], Sequence[Vertex]], neighbours_positive: tuple[Sequence[Vertex], Sequence[Vertex]]):
    """
      Connects a new node to its neighbours along the axis it was injected into a box by.
    This is a generalised implementation intended to used both when the inputs were created by injecting between segments of a resizable box and when by splitting an existing window.

    Note: negative/positive refer to the sign of directions along the axis. See the Axis documentation for details.


    Expected outcome (to write test for): the respective elements of neighbours_before and neighbours_after are connected by a line of 4 bidirectionally connected vertices such that only beginning and end of the line may have multiples.
        e.g.:
            neighbours_after[0] == neighbours_before[0].neighbours[axis.DIRECTIONS[-1]][-1].neighbours[axis.DIRECTIONS[-1]][0].neighbours[axis.DIRECTIONS[-1]][0]
            neighbours_after[1] == neighbours_before[1].neighbours[axis.DIRECTIONS[-1]][0].neighbours[axis.DIRECTIONS[-1]][0].neighbours[axis.DIRECTIONS[-1]][1]
        See associated unit tests for more examples
    :return: None
    :param target:
    :param parallel_axis:
    :param neighbours_negative: the neighbours along the negative edge of the target Window, with respect to the given axis
    :param neighbours_positive: the neighbours along the positive edge of the target Window, with respect to the given axis
    :return:
    """
    ## now connect the new tile to its neighbours along the split axis.
    for (each_dir, neighbours_in_dir, neighbours_in_opposite_dir) in zip(parallel_axis.directions, (neighbours_negative, neighbours_positive), (neighbours_positive, neighbours_negative)):
        for (each_new_vertex, each_intended_neighbours, each_opposite_neighbours) in zip(target.sides[each_dir], neighbours_in_dir, neighbours_in_opposite_dir):
            each_neighbour = each_intended_neighbours[0]
            each_other_neighbour = each_opposite_neighbours[0]
            each_new_vertex.neighbours[each_dir] = list(each_intended_neighbours)

            if len(each_neighbour.neighbours[each_dir.opposite]) == 0 or are_aligned(each_new_vertex, each_neighbour, parallel_axis.perpendicular):
                each_neighbour.neighbours[each_dir.opposite] = [each_new_vertex]
            elif each_neighbour.neighbours[each_dir.opposite][-1] == each_other_neighbour:
               each_neighbour.neighbours[each_dir.opposite][-1] = each_new_vertex
            if len(each_intended_neighbours) > 1:
                assert len(each_intended_neighbours) == 2

                each_neighbour = each_intended_neighbours[-1]
                each_other_neighbour = each_opposite_neighbours[0]
                if len(each_neighbour.neighbours[each_dir.opposite]) == 0 or are_aligned(each_new_vertex, each_neighbour, parallel_axis.perpendicular):
                    each_neighbour.neighbours[each_dir.opposite] = [each_new_vertex]
                elif each_neighbour.neighbours[each_dir.opposite][0] == each_other_neighbour:
                    each_neighbour.neighbours[each_dir.opposite][0] = each_new_vertex

def repair_connections_along_perpendicular_axis(perpendicular_axis: Axis, exterior_negative_edge: Edge, interior_negative_edge: Edge, exterior_positive_edge: Edge, interior_positive_edge: Edge):
    """

    :param perpendicular_axis:
    :param exterior_negative_edge:
    :param interior_negative_edge:
    :param exterior_positive_edge:
    :param interior_positive_edge:
    :return:
    """

    #first collect the line of nodes that were adjacent to the target on either side of the cross axis. These will not have been modified yet.
    parallel_axis = perpendicular_axis.perpendicular
    forward_parallel_direction: CardinalDirection = parallel_axis.directions[-1]


    for (each_outward_direction, each_exterior_edge, each_interior_edge) in zip(perpendicular_axis.directions, (exterior_negative_edge, exterior_positive_edge), (interior_negative_edge, interior_positive_edge)):
        each_outward_direction: CardinalDirection
        each_exterior_edge: Edge
        each_interior_edge: Edge

        cur_interior_part = each_interior_edge.a
        interior_parts = [cur_interior_part]
        while cur_interior_part is not each_interior_edge.b:
            next_neighbours = cur_interior_part.neighbours[forward_parallel_direction]
            assert len(next_neighbours) == 1
            cur_interior_part = next_neighbours[0]
            interior_parts.append(cur_interior_part)

        assert not interior_parts[0].is_sentinel
        assert not interior_parts[-1].is_sentinel

        assert each_exterior_edge.a.is_sentinel == each_exterior_edge.b.is_sentinel

        if each_exterior_edge.a.is_sentinel:
            #exterior sentinels case
            #sentinels need to be handled differently since otherwise we could break the exterior edge's connection to its actual nearest neighboura

            start_index = 0

            #for each interior corner which happens to be the edge of the wall, its only neighbour in that direction should be the corner, and they should be bidirectionnally connected. For all others, it should be both corners.
            if each_interior_edge.a.location == each_exterior_edge.a.location:
                each_interior_edge.a.neighbours[each_outward_direction] = [each_exterior_edge.a]
                each_exterior_edge.a.neighbours[each_outward_direction.opposite] = [each_interior_edge.a]
                start_index = 1

            end_index = None

            if each_interior_edge.b.location == each_exterior_edge.b.location:
                each_interior_edge.b.neighbours[each_outward_direction] = [each_exterior_edge.b]
                each_exterior_edge.b.neighbours[each_outward_direction.opposite] = [each_interior_edge.b]
                end_index = -1

            for each_internal_vertex in interior_parts[start_index:end_index]:
                each_internal_vertex.neighbours[each_outward_direction] = list(each_exterior_edge)

        else:
            #normal case
            #if the ends of the exterior edge align with the ends of the interior edge or the interior ends are closer to the exterior ends than the old neighbour (which could have potentially moved e.g. due to a split), update it
            if are_aligned(each_exterior_edge.a, each_interior_edge.a, parallel_axis):
                each_exterior_edge.a.neighbours[each_outward_direction.opposite] = [each_interior_edge.a]
            elif each_interior_edge.a.location[perpendicular_axis] < each_exterior_edge.a.neighbours[each_outward_direction.opposite][-1].location[perpendicular_axis]:
                each_exterior_edge.a.neighbours[each_outward_direction.opposite][-1] = each_interior_edge.a

            if are_aligned(each_exterior_edge.b, each_interior_edge.b, parallel_axis):
                each_exterior_edge.b.neighbours[each_outward_direction.opposite] = [each_interior_edge.b]
            elif each_interior_edge.b.location[perpendicular_axis] > each_exterior_edge.b.neighbours[each_outward_direction.opposite][0].location[perpendicular_axis]:
                each_exterior_edge.b.neighbours[each_outward_direction.opposite][0] = each_interior_edge.b

            cur_exterior_part = each_exterior_edge.a
            exterior_parts = [cur_exterior_part]
            while cur_exterior_part is not each_exterior_edge.b:
                next_neighbours = cur_exterior_part.neighbours[forward_parallel_direction]
                assert len(next_neighbours) == 1
                cur_exterior_part = next_neighbours[0]
                exterior_parts.append(cur_exterior_part)

            # todo: make the following loops more efficient
            # this currently scans the opposite side once for every node on the cuurrent side, which may be inefficient/unneccesary
            # one possible solution could be using an initial test to decide whether to go forward or backward through the opposing list, before looping in that direction.
            # although, and this may need to very verified or tested, in the case that the decision is to go backwards, we could perhaps just copy the neighbours given to the previous node without actually moving!

            # todo: write a symmetrical version that can be used for both (the external usage will need to supply the corresponding slice of the external parts)

            internal_index = 0

            for external_index in range(1, len(exterior_parts)-1):
                each_exterior_vertex = exterior_parts[external_index]
                old_neighbours = each_exterior_vertex.neighbours[each_outward_direction.opposite]
                candidate_neighbours = [None, None]

                internal_index = 0
                while internal_index < len(interior_parts):
                    each_interior_vertex = interior_parts[internal_index]
                    if are_aligned(each_exterior_vertex, each_interior_vertex, parallel_axis):
                        candidate_neighbours = [each_interior_vertex]
                        break
                    if (
                            each_exterior_vertex.location[parallel_axis] > each_interior_vertex.location[parallel_axis]
                                and not
                            each_exterior_vertex.location[parallel_axis] >= interior_parts[internal_index + 1].location[parallel_axis]
                    ):
                        #we want the LAST node less than us to be our 'left'
                        candidate_neighbours[0] = each_interior_vertex
                    elif each_exterior_vertex.location[parallel_axis] < each_interior_vertex.location[parallel_axis]:
                        #we want the FIRST node greater than us to be on our 'right'
                        candidate_neighbours[-1] = each_interior_vertex
                        break


                    internal_index += 1

                if len(candidate_neighbours) > 1 and len(old_neighbours) == 2:
                    candidate_neighbours = [each_new if each_new is not None else each_old for (each_new, each_old) in zip(candidate_neighbours, old_neighbours)]
                assert None not in candidate_neighbours

                each_exterior_vertex.neighbours[each_outward_direction.opposite] = candidate_neighbours

            external_index = 0
            for internal_index in range(len(interior_parts)):
                each_interior_vertex = interior_parts[internal_index]
                old_neighbours = interior_parts[internal_index].neighbours[each_outward_direction]
                candidate_neighbours = [None, None]

                external_index = 0
                while external_index < len(exterior_parts):
                    each_exterior_vertex = exterior_parts[external_index]
                    if are_aligned(each_interior_vertex, each_exterior_vertex, parallel_axis):
                        # we want the LAST node this is greater than to be our 'left'
                        candidate_neighbours = [each_exterior_vertex]
                        break
                    elif (
                            each_interior_vertex.location[parallel_axis] > each_exterior_vertex.location[parallel_axis]
                                and not
                            each_interior_vertex.location[parallel_axis] >= exterior_parts[external_index + 1].location[parallel_axis]
                    ):
                        # we want the LAST node less than us to be our 'left'
                        candidate_neighbours[0] = each_exterior_vertex
                    elif each_interior_vertex.location[parallel_axis] < each_exterior_vertex.location[parallel_axis]:
                        # we want the FIRST node greater than us to be on our 'right'
                        candidate_neighbours[-1] = each_exterior_vertex
                        break

                    external_index += 1

                if len(candidate_neighbours) > 1 and len(old_neighbours) == 2:
                    candidate_neighbours = [each_new if each_new is not None else each_old for (each_new, each_old) in zip(candidate_neighbours, old_neighbours)]
                assert None not in candidate_neighbours

                each_interior_vertex.neighbours[each_outward_direction] = candidate_neighbours

def fill_wall_with_new_window(manager: GeometricTileManager, target: Wall) -> Window:
    """
    Note: although most procedures for creating Windows can technically succeed but return results that show problems (constraint violations), this procedure throws a ValueError if the wall is already divided (has Windows inside).
        It is intended to be used as part of a broader procedure that handles these errors.
    :param graphMgr:
    :param target:
    :return:
    """
    if is_divided(target):
        raise ValueError(f"Wall {target.generate_tag()} is not empty.")
    else:
        result = manager.graph.create_tile(Window, *map(lambda x: x.location, target.corners))
        for (each_result_corner, each_target_corner) in zip(result.corners, target.corners):
            for each_dir in each_result_corner.role:
                each_result_corner.neighbours[each_dir] = [each_target_corner]
                each_target_corner.neighbours[each_dir.opposite] = [each_result_corner]

        return result

def split_window_with_new_window(manager: GeometricTileManager, target_tile: Window, target_direction: CardinalDirection) -> tuple[Window, Sequence[StateProblem]]:
    """
    This implementation splits a specific side, but user-facing behaviour is intended to use configurations to allow default splitting direction (and, for example, to make room within a containing resizable box).
        This calculation will probably be extracted to a common space later, but for now note that available space allocated as follows:
        - The desired outer dimensions of the requested resizable box are always met exactly, with residuals distributed as fairly as possible (preferencing smaller contents)
        - First: Starting from the split origin and its immediate adjacent margins (or top/left if no origin is given) and then from both of the the outside of the supposed resizable box inward: each window is given at least 1px and each margin is given full allocation until no space
        - The remaining space is allocated by scaling the contents to fit it, rounded down, and with any modulo remainder allocated to smallest segments first, or otherwise left-to-right/top-to-bottom.
    :param target_tile:
    :param target_direction: The side on which the new window will be created. The old window will be split in half along the parallel axis and the new window will be placed on this side of the old one.
    :return: The created window, and a list of state problems (violated constraints on minimum sizes or margins)
    """

    #todo: deal with minima, for now just evenly split allocating excess to the new container, minimum viable product and all.

    ##start by recording the relevant neighbours along the split axis and the cross axis.

    #aligned neighbours may be multiple per vertex but the cross neighbours are always exactly one)
    # i.e. aligned_neighbour[i] = pair[list[Vertex]], cross_neighbours_x = pair[Vertex]
    # more specifically, aligned neighbours are the vertices on either side of where the new tile was injected, including the tile that was split and its neighbours on the split side.
    parallel_neighbours: tuple[tuple[Sequence[Vertex], Sequence[Vertex]], tuple[Sequence[Vertex], Sequence[Vertex]]]
    parallel_neighbours = (
            tuple(list([each_vertex] for each_vertex in target_tile.sides[target_direction])),
            tuple(list(list(each_vertex.neighbours[target_direction]) for each_vertex in target_tile.sides[target_direction]))
        )
    if not target_direction.is_positive:
        parallel_neighbours = (parallel_neighbours[1], parallel_neighbours[0])


    #These are the outermost neighbours on the sides to the split direction.
    # E.g. if the split was west or east, the negative start/end would be the west-most/east-most neighbour vertices of the northWest/northWest corners of the box that will be modified (in this case, a window), and the positive start/end are the corresponding vertices to the south.
    negative_perpendicular_exterior_edge: Edge
    positive_perpendicular_exterior_edge: Edge
    (negative_perpendicular_exterior_edge, positive_perpendicular_exterior_edge) = [
        Edge(*(each_corner.neighbours[each_perpendicular_direction][i] for (i,each_corner) in zip((0,-1), target_tile.sides[each_perpendicular_direction])))
        for each_perpendicular_direction in target_direction.axis.perpendicular.directions
    ]

    ##now make space for the new box and create it, before calling the function that correct
    #note: in a resizable box this may need to be sorted?
    old_target_edge: Edge = target_tile.sides[target_direction]
    old_opposite_edge: Edge = target_tile.sides[target_direction.opposite]
    start_edge: Edge
    end_edge: Edge
    result: Window
    (start_edge, end_edge) = [target_tile.sides[each] for each in target_direction.axis.directions]

    available_space = Edge(start_edge.a, end_edge.a).distance()

    num_segments = 2

    problems = []

    if available_space < num_segments:
        # outcome: the windows just all occupy the full available space instead of allocating them in any particular way; This will not break relative neighbouring since each node is connected to the first/last node to be on either side of it as needed.

        problems.append(BoxTooSmallForMarginsProblem(target_tile))

        result = manager.graph.create_tile(Window, {each.role: each.location for each in target_tile.corners})

    elif available_space < (num_segments + manager.settings.static_config.constraints.window_margin):
        #outcome: the windows occupy singular pixels at the the ends of the space, and any remainder is margin.
        #but more generally: this where we would allocate margins and spaces individually as needed and result a problem like "toosmallformargins"

        problems.append(BoxTooSmallForMarginsProblem(target_tile))

        result = manager.graph.create_tile(Window, {
            old_target_edge.a.role: old_target_edge.a.location,
            old_target_edge.b.role: old_target_edge.b.location,
            old_opposite_edge.a.role: old_target_edge.a.location - target_direction.unit_vector,
            old_opposite_edge.b.role: old_target_edge.b.location - target_direction.unit_vector,
        })

        #update old location
        old_target_edge.a.location = old_opposite_edge.a.location + target_direction.unit_vector
        old_target_edge.b.location = old_opposite_edge.b.location + target_direction.unit_vector
    else:
        #outcome: this is where we allocate margins first and then consider minimum sizes for the remainder.

        available_space -= manager.settings.static_config.constraints.window_margin

        base_size = available_space//2
        remainder = available_space % 2

        if target_direction.is_positive:
            old_size_vector = target_direction.unit_vector * (base_size + remainder)
            new_size_vector = target_direction.unit_vector * base_size
        else:
            old_size_vector = target_direction.unit_vector * base_size
            new_size_vector = target_direction.unit_vector * (base_size + remainder)

        result = manager.graph.create_tile(Window, {
            old_target_edge.a.role: old_target_edge.a.location,
            old_target_edge.b.role: old_target_edge.b.location,
            old_opposite_edge.a.role: old_target_edge.a.location - new_size_vector,
            old_opposite_edge.b.role: old_target_edge.b.location - new_size_vector,
        })

        #update old locations
        old_target_edge.a.location = old_opposite_edge.a.location + old_size_vector
        old_target_edge.b.location = old_opposite_edge.b.location + old_size_vector

    establish_connections_along_injection_axis(result, target_direction.axis, *parallel_neighbours)


    #Now deal with cross axis connections.
    #to do this, we need to determine the interior start/ends. Whilst the exterior includes neighbours past the boundaries of the modified box, the interior start/ends are the vertices that define the boundaries of the new box (in this case, the outer corners of the box enclosing the region covered by the ld and new windows.
    negative_perpendicular_interior_edge: Edge
    positive_perpendicular_interior_edge: Edge

    (negative_perpendicular_interior_edge, positive_perpendicular_interior_edge) = [
        Edge(target_tile.sides[each_perpendicular_direction].a, result.sides[each_perpendicular_direction].b)
        for each_perpendicular_direction in target_direction.axis.perpendicular.directions
    ] if target_direction.is_positive else [
        Edge(result.sides[each_perpendicular_direction].a, target_tile.sides[each_perpendicular_direction].b)
        for each_perpendicular_direction in target_direction.axis.perpendicular.directions
    ]

    repair_connections_along_perpendicular_axis(target_direction.axis.perpendicular, negative_perpendicular_exterior_edge, negative_perpendicular_interior_edge, positive_perpendicular_exterior_edge, positive_perpendicular_interior_edge)


    return (result, problems)