from display.demo.graph_renderer import render_state
from procedures.manipulation import fill_canvas_with_new_window
from structures.graph import Window, Canvas
from structures.GeometricTileManager import GeometricTileManager

from structures.geomtry.Vector import Vector


def basic_canvas_and_window_rendering_test():
    gtmInstance = GeometricTileManager()

    first_canvas = gtmInstance.graph.create_tile(Canvas, Vector(100, 100), Vector(300, 300))

    gtmInstance.graph.create_tile(Window, Vector(150, 150), Vector(100, 100))
    gtmInstance.graph.create_tile(Window, Vector(250, 250), Vector(100, 100))

    second_canvas = gtmInstance.graph.create_tile(Canvas, Vector(500, 100), Vector(300, 300))

    fill_canvas_with_new_window(gtmInstance, second_canvas)

    render_state(gtmInstance, Vector(1000, 1000))

if __name__ == '__main__':
    """
    Execute all tests in this file
    """
    basic_canvas_and_window_rendering_test()