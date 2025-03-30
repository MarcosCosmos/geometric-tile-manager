import unittest

from procedures.examination import is_divided
from procedures.manipulation import establish_connections_along_injection_axis
from geometry import diagonal_direction
from geometry.direction.constants import *
from manager import GeometricTileManager


class Cases(unittest.TestCase):
    def test_single_row_of_flush_windows(self):
        gtmInstance = GeometricTileManager()

        first_window = gtmInstance.graph.create_tile(Window, Vector(100, 100), Vector(100, 100))
        second_window = gtmInstance.graph.create_tile(Window, Vector(300, 100), Vector(100, 100))
        third_window = gtmInstance.graph.create_tile(Window, Vector(500, 100), Vector(100, 100))
        all_windows = [first_window, second_window, third_window]

        establish_connections_along_injection_axis(second_window, Axis.HORIZONTAL, ([first_window.corners[NORTH_EAST]], [first_window.corners[SOUTH_EAST]]), ([third_window.corners[NORTH_WEST]], [third_window.corners[SOUTH_WEST]]))


        for each_window in all_windows:
            self.assertFalse(is_divided(each_window))

        for each_window in all_windows[:-1]:
            for each_corner in each_window.corners:
                self.assertEqual(len(each_corner.neighbours[EAST]), 1)
        for each_window in all_windows[1:]:
            for each_corner in each_window.corners:
                self.assertEqual(len(each_corner.neighbours[WEST]), 1)

        for i in range(len(all_windows)-1):
            each_window = all_windows[i]
            next_window = all_windows[i+1]
            for each_dir in Axis.VERTICAL.directions:
                self.assertIs(each_window.corners[DiagonalDirection((EAST, each_dir))].neighbours[EAST][0], next_window.corners[DiagonalDirection((WEST, each_dir))])

        for i in range(1, len(all_windows)):
            each_window = all_windows[i]
            previous_window = all_windows[i-1]
            for each_dir in Axis.VERTICAL.directions:
                self.assertIs(each_window.corners[DiagonalDirection((WEST, each_dir))].neighbours[WEST][0], previous_window.corners[DiagonalDirection((EAST, each_dir))])

    def test_single_column_of_flush_windows(self):
        gtmInstance = GeometricTileManager()

        first_window = gtmInstance.graph.create_tile(Window, Vector(100, 100), Vector(100, 100))
        second_window = gtmInstance.graph.create_tile(Window, Vector(100, 300), Vector(100, 100))
        third_window = gtmInstance.graph.create_tile(Window, Vector(100, 500), Vector(100, 100))
        all_windows = [first_window, second_window, third_window]

        establish_connections_along_injection_axis(second_window, Axis.VERTICAL, ([first_window.corners[SOUTH_WEST]], [first_window.corners[SOUTH_EAST]]), ([third_window.corners[NORTH_WEST]], [third_window.corners[NORTH_EAST]]))


        for each_window in all_windows:
            self.assertFalse(is_divided(each_window))

        for each_window in all_windows[:-1]:
            for each_corner in each_window.corners:
                self.assertEqual(len(each_corner.neighbours[SOUTH]), 1)
        for each_window in all_windows[1:]:
            for each_corner in each_window.corners:
                self.assertEqual(len(each_corner.neighbours[NORTH]), 1)

        for i in range(len(all_windows)-1):
            each_window = all_windows[i]
            next_window = all_windows[i+1]
            for each_dir in Axis.HORIZONTAL.directions:
                self.assertIs(each_window.corners[DiagonalDirection((each_dir, SOUTH))].neighbours[SOUTH][0], next_window.corners[DiagonalDirection((each_dir, NORTH))])

        for i in range(1, len(all_windows)):
            each_window = all_windows[i]
            previous_window = all_windows[i-1]
            for each_dir in Axis.HORIZONTAL.directions:
                self.assertIs(each_window.corners[DiagonalDirection((each_dir, NORTH))].neighbours[NORTH][0], previous_window.corners[DiagonalDirection((each_dir, SOUTH))])

if __name__ == '__main__':
    unittest.main()
