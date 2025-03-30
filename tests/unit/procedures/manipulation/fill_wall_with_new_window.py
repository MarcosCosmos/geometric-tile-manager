import unittest

from procedures.manipulation import fill_wall_with_new_window
from structures.geomtry.CardinalDirection import CardinalDirection
from structures.geomtry.Vector import Vector
from structures.graph import Wall
from structures.GeometricTileManager import GeometricTileManager
from structures.geomtry.DiagonalDirection import DiagonalDirection


class MyTestCase(unittest.TestCase):
    def test_solitary_wall(self):
        gtmInstance = GeometricTileManager()
        first_wall = gtmInstance.graph.create_tile(Wall, Vector(100, 100), Vector(300, 300))

        first_window = fill_wall_with_new_window(gtmInstance, first_wall)
        for each_direction in DiagonalDirection:
            each_direction: DiagonalDirection
            each_wall_corner = first_wall.corners[each_direction]
            each_window_corner = first_window.corners[each_direction]
            for each_cardinal_component in each_direction.value:
                each_cardinal_component: CardinalDirection
                self.assertEqual(len(each_wall_corner.neighbours[each_cardinal_component.opposite]), 1)
                self.assertIs(each_wall_corner.neighbours[each_cardinal_component.opposite][0], each_window_corner)

                self.assertEqual(len(each_window_corner.neighbours[each_cardinal_component]), 1)
                self.assertIs(each_window_corner.neighbours[each_cardinal_component][0], each_wall_corner)

if __name__ == '__main__':
    unittest.main()
