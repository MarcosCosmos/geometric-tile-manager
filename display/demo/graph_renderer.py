import tkinter as tk
from tkinter import Tk, Canvas
from structures.geomtry.Axis import Axis
from structures.manager import GeometricTileManager
from structures.geomtry.Vector import Vector
from structures.graph import Wall, Window


def render_state(target: GeometricTileManager, canvasSize: Vector[int], show_links: bool = False):
    """
    This demo renders a single window with (potentially) multiple Walls, but real-world usage will involve one window per wall.
    :param target:
    :return:
    """
    root = Tk()

    theCanvas = Canvas(root, bg="white", height=canvasSize.vertical, width=canvasSize.horizontal)

    for each_wall in target.graph.by_type[Wall].values():
        theCanvas.create_rectangle(
            each_wall.corners.north_west.location.horizontal,
            each_wall.corners.north_west.location.vertical,
            each_wall.corners.south_east.location.horizontal,
            each_wall.corners.south_east.location.vertical,
            outline=target.settings.static_config.render.wall_border_color,
            width=target.settings.static_config.render.wall_border_width,
        )
    for each_window in target.graph.by_type[Window].values():
        theCanvas.create_rectangle(
            each_window.corners.north_west.location.horizontal,
            each_window.corners.north_west.location.vertical,
            each_window.corners.south_east.location.horizontal,
            each_window.corners.south_east.location.vertical,
            outline=target.settings.static_config.render.window_border_color,
            width=target.settings.static_config.render.window_border_width
        )
    if show_links:
        for each_wall in target.graph.by_type[Wall].values():
            for each_corner in each_wall.corners:
                each_diagonal = each_corner.role
                for each_cardinal in each_diagonal.opposite.value:
                    each_axis = each_cardinal.axis
                    each_neighbour = each_corner.neighbours[each_cardinal][0]
                    theCanvas.create_line(
                        *(Vector._make({
                            each_axis: each_corner.location[each_axis],
                            each_axis.perpendicular: each_neighbour.location[each_axis.perpendicular]
                        })),
                        *(Vector._make({
                            each_axis: each_neighbour.location[each_axis] + (1 if each_cardinal.is_positive else -1),
                            each_axis.perpendicular: each_neighbour.location[each_axis.perpendicular]
                        })),
                        fill=target.settings.static_config.render.sentinel_edge_color,
                        dash=target.settings.static_config.render.edge_dash,
                        arrow=tk.LAST,
                        width=target.settings.static_config.render.window_border_width
                    )
        for each_window in target.graph.by_type[Window].values():
            for each_axis in Axis:
                each_axis: Axis
                each_forward_direction = each_axis.directions[1]
                for each_corner in each_window.sides[each_forward_direction]:
                    each_neighbours = each_corner.neighbours[each_forward_direction]
                    if len(each_neighbours) == 0:
                        ...
                    elif len(each_neighbours) == 2 and each_neighbours[0].location[each_axis] == each_neighbours[-1].location[each_axis]:
                        theCanvas.create_line(
                            *each_corner.location,
                            *(Vector._make({
                                each_axis: each_neighbours[1].location[each_axis] + 1,
                                each_axis.perpendicular: each_corner.location[each_axis.perpendicular]
                            })),
                            fill=(
                                target.settings.static_config.render.sentinel_edge_color
                                    if each_neighbours[0].is_sentinel else
                                target.settings.static_config.render.edge_color
                            ),
                            dash=target.settings.static_config.render.edge_dash,
                            arrow=tk.LAST,
                            width=target.settings.static_config.render.window_border_width
                        )
                    else:
                        for each_neighbour in each_neighbours:
                            # theCanvas.create_line(*each_corner.location, *each_neighbour.location, fill=target.settings.static_config.render.edge_color,
                            #                       dash=target.settings.static_config.render.edge_dash, arrow=tk.LAST)

                            theCanvas.create_line(
                                *each_corner.location,
                                *(Vector._make({
                                    each_axis: each_neighbour.location[each_axis] + (1 if each_neighbour.location[each_axis] == each_corner.location[each_axis] else 0),
                                    each_axis.perpendicular: each_neighbour.location[each_axis.perpendicular]
                                })),
                                fill=(
                                    target.settings.static_config.render.sentinel_edge_color
                                    if each_neighbour.is_sentinel else
                                    target.settings.static_config.render.edge_color
                                ),
                                dash=target.settings.static_config.render.edge_dash,
                                arrow=tk.LAST,
                                width=target.settings.static_config.render.window_border_width
                            )

                each_backward_direction = each_axis.directions[0]
                for each_corner in each_window.sides[each_backward_direction]:
                    each_neighbours = each_corner.neighbours[each_backward_direction]
                    if len(each_neighbours) == 0:
                        ...
                    elif len(each_neighbours) == 2 and each_neighbours[0].location[each_axis] == each_neighbours[-1].location[each_axis]:
                        assert each_neighbours[0] is not each_neighbours[-1]
                        theCanvas.create_line(
                            *each_corner.location,
                            *(Vector._make({
                                each_axis: each_neighbours[0].location[each_axis] - 1,
                                each_axis.perpendicular: each_corner.location[each_axis.perpendicular]
                            })),
                            fill=(
                                target.settings.static_config.render.sentinel_edge_color
                                    if each_neighbours[0].is_sentinel else
                                target.settings.static_config.render.edge_color),
                            dash=target.settings.static_config.render.edge_dash,
                            arrow=tk.LAST,
                            width=target.settings.static_config.render.window_border_width
                        )
                    else:
                        assert len(each_neighbours) == 1 or each_neighbours[0] is not each_neighbours[-1]
                        for each_neighbour in each_neighbours:
                            # theCanvas.create_line(*each_corner.location, *each_neighbour.location, fill=target.settings.static_config.render.edge_color,
                            #                       dash=target.settings.static_config.render.edge_dash, arrow=tk.LAST)

                            theCanvas.create_line(
                                *each_corner.location,
                                *(Vector._make({
                                    each_axis: each_neighbour.location[each_axis] - (1 if each_neighbour.location[each_axis] == each_corner.location[each_axis] else 0),
                                    each_axis.perpendicular: each_neighbour.location[each_axis.perpendicular]
                                })),
                                fill=(
                                    target.settings.static_config.render.sentinel_edge_color
                                        if each_neighbour.is_sentinel else
                                    target.settings.static_config.render.edge_color
                                ),
                                dash=target.settings.static_config.render.edge_dash,
                                arrow=tk.LAST,
                                width=target.settings.static_config.render.window_border_width
                        )

    theCanvas.pack()

    root.mainloop()