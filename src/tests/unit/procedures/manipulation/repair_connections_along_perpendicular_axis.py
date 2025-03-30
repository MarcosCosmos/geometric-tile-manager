import unittest

from geometry.direction.cardinal import CardinalDirection
from geometry.graph.canvas import Canvas
from geometry.graph.edge import Edge
from geometry.graph.vertex import Vertex
from geometry.graph.window import Window
from geometry.vector import Vector
from procedures.manipulation import establish_connections_along_injection_axis, \
    repair_connections_along_perpendicular_axis
from geometry.direction.constants import *
from manager import GeometricTileManager


class MyTestCase(unittest.TestCase):

    def test_single_row_in_canvas(self):
        """
        This test is only valid under the assumption that establish_connections_along_injection_axis is working correctly, as it depends on that to link the windows horizontally prior to vertical repair.
        :return:
        """
        gtmInstance = GeometricTileManager()

        first_window = gtmInstance.graph.create_tile(Window, Vector(100, 100), Vector(100, 100))
        second_window = gtmInstance.graph.create_tile(Window, Vector(300, 100), Vector(100, 100))
        third_window = gtmInstance.graph.create_tile(Window, Vector(500, 100), Vector(100, 100))

        first_canvas = gtmInstance.graph.create_tile(Canvas, first_window.corners.north_west.location, third_window.corners.north_east.location, third_window.corners.south_east.location, first_window.corners.south_west.location)

        all_windows = [first_window, second_window, third_window]

        establish_connections_along_injection_axis(first_window, HORIZONTAL, ([first_canvas.corners.north_west], [first_canvas.corners.south_west]), ([second_window.corners.north_west], [second_window.corners.south_west]))

        establish_connections_along_injection_axis(third_window, HORIZONTAL, ([second_window.corners.north_east], [second_window.corners.south_east]), ([first_canvas.corners.north_east], [first_canvas.corners.south_east]))


        negative_perpendicular_exterior_edge: Edge = first_canvas.sides.north
        positive_perpendicular_exterior_edge: Edge = first_canvas.sides.south

        negative_perpendicular_interior_edge: Edge = Edge(first_window.corners.north_west, third_window.corners.north_east)
        positive_perpendicular_interior_edge: Edge = Edge(first_window.corners.south_west, third_window.corners.south_east)

        repair_connections_along_perpendicular_axis(VERTICAL,
                                                    negative_perpendicular_exterior_edge,
                                                    negative_perpendicular_interior_edge,
                                                    positive_perpendicular_exterior_edge,
                                                    positive_perpendicular_interior_edge)

        outer_corners = (first_window.corners.north_west, third_window.corners.north_east, third_window.corners.south_east, first_window.corners.south_west)
        for each_window_corner in outer_corners:
            each_direction = each_window_corner.role
            each_canvas_corner = first_canvas.corners[each_direction]
            for each_cardinal_component in each_direction.value:
                each_cardinal_component: CardinalDirection
                self.assertEqual(len(each_canvas_corner.neighbours[each_cardinal_component.opposite]), 1)
                self.assertIs(each_canvas_corner.neighbours[each_cardinal_component.opposite][0], each_window_corner)

                self.assertEqual(len(each_window_corner.neighbours[each_cardinal_component]), 1)
                self.assertIs(each_window_corner.neighbours[each_cardinal_component][0], each_canvas_corner)

        upper_inner_vertices: list[Vertex] = []

        current = first_window.corners.north_west
        while current is not third_window.corners.north_west:
            current = current.neighbours.east[0]
            upper_inner_vertices.append(current)

        for each_vertex in upper_inner_vertices:
            self.assertSequenceEqual(each_vertex.neighbours.north, first_canvas.sides.north)

        lower_inner_vertices: list[Vertex] = []
        current = first_window.corners.south_west
        while current is not third_window.corners.south_west:
            current = current.neighbours.east[0]
            lower_inner_vertices.append(current)


        for each_vertex in lower_inner_vertices:
            self.assertSequenceEqual(each_vertex.neighbours.south, first_canvas.sides.south)


if __name__ == '__main__':
    unittest.main()
