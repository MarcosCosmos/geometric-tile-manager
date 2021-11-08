import unittest

from procedures.manipulation import establish_connections_along_injection_axis, repair_connections_along_perpendicular_axis
from structures.graph import Edge, Vertex, Window, Wall
from structures.manager import GeometricTileManager
from structures.geometry import *


class MyTestCase(unittest.TestCase):

    def test_single_row_in_wall(self):
        """
        This test is only valid under the assumption that establish_connections_along_injection_axis is working correctly, as it depends on that to link the windows horizontally prior to vertical repair.
        :return:
        """
        gtmInstance = GeometricTileManager()

        first_window = gtmInstance.graph.create_tile(Window, Vector(100, 100), Vector(100, 100))
        second_window = gtmInstance.graph.create_tile(Window, Vector(300, 100), Vector(100, 100))
        third_window = gtmInstance.graph.create_tile(Window, Vector(500, 100), Vector(100, 100))

        first_wall = gtmInstance.graph.create_tile(Wall, first_window.corners.north_west.location, third_window.corners.north_east.location, third_window.corners.south_east.location, first_window.corners.south_west.location)

        all_windows = [first_window, second_window, third_window]

        establish_connections_along_injection_axis(first_window, HORIZONTAL, ([first_wall.corners.north_west], [first_wall.corners.south_west]), ([second_window.corners.north_west], [second_window.corners.south_west]))

        establish_connections_along_injection_axis(third_window, HORIZONTAL, ([second_window.corners.north_east], [second_window.corners.south_east]), ([first_wall.corners.north_east], [first_wall.corners.south_east]))


        negative_perpendicular_exterior_edge: Edge = first_wall.sides.north
        positive_perpendicular_exterior_edge: Edge = first_wall.sides.south

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
            each_wall_corner = first_wall.corners[each_direction]
            for each_cardinal_component in each_direction.value:
                each_cardinal_component: CardinalDirection
                self.assertEqual(len(each_wall_corner.neighbours[each_cardinal_component.opposite]), 1)
                self.assertIs(each_wall_corner.neighbours[each_cardinal_component.opposite][0], each_window_corner)

                self.assertEqual(len(each_window_corner.neighbours[each_cardinal_component]), 1)
                self.assertIs(each_window_corner.neighbours[each_cardinal_component][0], each_wall_corner)

        upper_inner_vertices: list[Vertex] = []

        current = first_window.corners.north_west
        while current is not third_window.corners.north_west:
            current = current.neighbours.east[0]
            upper_inner_vertices.append(current)

        for each_vertex in upper_inner_vertices:
            self.assertSequenceEqual(each_vertex.neighbours.north, first_wall.sides.north)

        lower_inner_vertices: list[Vertex] = []
        current = first_window.corners.south_west
        while current is not third_window.corners.south_west:
            current = current.neighbours.east[0]
            lower_inner_vertices.append(current)


        for each_vertex in lower_inner_vertices:
            self.assertSequenceEqual(each_vertex.neighbours.south, first_wall.sides.south)


if __name__ == '__main__':
    unittest.main()
