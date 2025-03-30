from display.demo.graph_renderer import render_state
from procedures.manipulation import fill_canvas_with_new_window, split_window_with_new_window
from geometry.direction.constants import *
from manager import GeometricTileManager


def basic_window_splitting_test():
    gtmInstance = GeometricTileManager()
    gtmInstance.settings.static_config.constraints.window_margin = 30

    first_canvas = gtmInstance.graph.create_tile(Canvas, Vector(100, 100), Vector(300, 300))

    first_window = fill_canvas_with_new_window(gtmInstance, first_canvas)

    try:
        fill_canvas_with_new_window(gtmInstance, first_canvas)
    except ValueError as err:
        print('Caught expected exception, details: ', err)

    (second_window, _) = split_window_with_new_window(gtmInstance, first_window, EAST)
    (third_window, _) = split_window_with_new_window(gtmInstance, second_window, EAST)
    (fourth_window, _) = split_window_with_new_window(gtmInstance, third_window, NORTH)
    (fifth_window, _) = split_window_with_new_window(gtmInstance, fourth_window, NORTH)
    (sixth_window, _) = split_window_with_new_window(gtmInstance, second_window, SOUTH)
    (seventh_window, _) = split_window_with_new_window(gtmInstance, second_window, NORTH)
    (eighth_window, _) = split_window_with_new_window(gtmInstance, second_window, NORTH)
    (ninth_window, _) = split_window_with_new_window(gtmInstance, sixth_window, NORTH)
    (tenth_window, _) = split_window_with_new_window(gtmInstance, first_window, WEST)
    (eleventh_window, _) = split_window_with_new_window(gtmInstance, tenth_window, NORTH)
    (twelfth_window, _) = split_window_with_new_window(gtmInstance, tenth_window, SOUTH)
    (thirteenth_window, _) = split_window_with_new_window(gtmInstance, eleventh_window, SOUTH)
    (fourteenth_window, _) = split_window_with_new_window(gtmInstance, eleventh_window, NORTH)

    gtmInstance.graph.name_tile(first_window, 'heya')

    gtmInstance.graph.name_tile(second_window, 'heya')

    for each_corner in first_canvas.corners:
        for each_cardinal in each_corner.role.opposite.value:
            each_neighbour = each_corner.neighbours[each_cardinal][0]
            assert each_neighbour.location == each_corner.location

    #todo: move tag lookup tests to unit tests

    print(geometry.graph.helpers.parse_tag(str(first_window.generate_tag())))
    print(second_window.generate_tag())
    print(
        geometry.graph.helpers.parse_tag(str(gtmInstance.graph[second_window.sides[EAST].generate_tag()].generate_tag())))
    print(geometry.graph.helpers.parse_tag(str(gtmInstance.graph[second_window.corners.north_west.generate_tag()].generate_tag())))

    #todo: write up automated unit test version.. will use an auto-generated set of splits
    test = gtmInstance.graph[first_window.generate_tag()]

    # gtmInstance.graph._erase_tile(TileTag('heya'))

    render_state(gtmInstance, Vector(1000, 1000), show_links=True)

    ...

if __name__ == '__main__':
    """
    Execute all tests in this file
    """
    basic_window_splitting_test()