from display.demo.graph_renderer import render_state
from procedures.manipulation import fill_wall_with_new_window
from structures.graph import Window, Wall
from structures.manager import GeometricTileManager

from structures.geometry import Vector


def basic_wall_and_window_rendering_test():
    gtmInstance = GeometricTileManager()

    first_wall = gtmInstance.graph.create_tile(Wall, Vector(100, 100), Vector(300, 300))

    gtmInstance.graph.create_tile(Window, Vector(150, 150), Vector(100, 100))
    gtmInstance.graph.create_tile(Window, Vector(250, 250), Vector(100, 100))

    second_wall = gtmInstance.graph.create_tile(Wall, Vector(500, 100), Vector(300, 300))

    fill_wall_with_new_window(gtmInstance, second_wall)

    render_state(gtmInstance, Vector(1000, 1000))

if __name__ == '__main__':
    """
    Execute all tests in this file
    """
    basic_wall_and_window_rendering_test()