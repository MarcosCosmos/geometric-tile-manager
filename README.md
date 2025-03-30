# The Geometric Tile Manager
## What is it?
A library of algorithms for hybrid dynamic/manual tiling window management in which decisions are based entirely on visible tile geometry. 

#### Why?
The main goal of the GTM project is to demonstrate a form of tile management which is both highly versatile and strictly follows the principle of least astonishment by ensuring that all visually identical layouts are also functionally equivalent. Unlike many other dynamic tiling systems, which rely on hierarchical tile splitting, GTM stores layouts as an axis-aligned graph of adjacent tiles, such that every tile is linked to its immediate neighbours in each cardinal direction, and there are no parent-child relationships. This allows arbitrary rectangular regions of the layout to be freely manipulated in any cardinal direction at any time.

#### What is it not?
A window manager, strictly speaking.

This initial project is a proof-of-concept implementation. It does not implement any OS interactions, and only provides demonstrative visualisations. The plan is to provide core functionality along with an extensible API. Usage for end users will be enabled via companion modules in a separate repository that expose IPC protocols for command line scripting and potential integration with more mature frameworks like [awesome][awesome].

## How?

As mentioned above, tiles are stored in an axis-aligned adjacency graph. Layout modifications are made with respect to an axis-aligned-bounding-box (AABB) region of the graph and act along one axis at a time. First, the AABB is recursively subdivided into row and/or column segments, depending on the desired axis and internal layout. Once the AABB has been selected and segmented, resizing any one segment (or series of connected segments) is as easy as scaling the remaining segments to free up or occupy space as necessary.

### Creating and Resizing Tiles
When requesting a resize action, users can specify the corresponding AABB to achieve manual tiling, or allow GTM to compute the AABB on a dynamic and a configurable basis.

E.g. when creating a new tile, you could make room for it by:
- Splitting a target (e.g. focused) tile in half (similar to [BSPWM][bspwm]), or
- Scaling an entire row/column of parallel tiles, inferred from those adjacent to a target tile (similar to [i3][i3]), or
- Select between these two options dynamically based on configured preferences or scripts.

### Automatically Computing AABBs
Automatically determining appropriate AABBs for most actions is straightforward.

Suppose we want to resize a tile by moving it's northern side vertically. In general, there 4 possibilities when looking for a suitable AABB:
1. There exists a column of tiles which can be drawn in parallel to the sides of the target window, extending in the target direction (north), and is *clean*.
   - *Clean*, as in: it includes at least one other tile and does not cut through the interior of any tiles.
   - In which case, that *clean* column is a suitable AABB.
2. The target tile borders multiple tiles on the northern side. These tiles would necessarily be dragged along with the target tile, so we can look for a wider column parallel to outer sides of those adjacent tiles.
   - Equally, any tiles bordering the southern side of the aforementioned neighbours so be included. This continues until a *clean* column is found. 
3. The target tile already occupies all of the available vertical space, so there is nothing to.
4. The tile has neighbours which could make room, but they do not form a *clean* column (pathological cases).

The above logic applies equally to rows and columns, and accounts for most cases when creating, destroying, or resizing tiles. The fourth, pathological case, is discussed in the next section.

### Pathological Cases
Whilst most layouts form grids, it is possible to create pathological layouts in which space is available, but a row/column cannot be formed without cutting through the interior of one or more tiles.

These situations are managed by selecting a prospective AABB that would cut through tiles, and then temporarily adjusting the overhanging tiles to fit them into the AABB. Temporary tiles are created to reserve the overhanging space, which is returned to the affected tiles after the desired adjustments are completed. The exact nature of these artificial adjustments will depend on the needs of the case.

A straightforward example of this is the special case in which the adjacent tiles in all four directions lie across the corners of the target tile in a circular fashion. 

When deleting a tile, for example, it is generally preferable to fill the space with an existing row or column if any is available. In the situation described above however, all four row/column candidates would cut through at least one neighbour and would also be asymmetrical. Unless a direction in which to donate the space has been specified, the fairest option would be to re-arrange the surrounding tiles into a grid split down the center of the deleted tile, such that tiles furthest to each corner of the grid make up the corresponding quadrant. This is achieved by creating four artificial tiles such that each artificial tile starts flush with a of the deleted target. The target tile can now be deleted donating the space to any one of the four artificial tiles. The unchanged artificial tiles along the same axis donates its space to the larger artificial first. Then, the cross-axis artificial tiles are removed, followed by the remaining artificial tile, such that a grid is formed.

### Deleting Tiles
In most cases, deleting a tile can be done trivially along one axis or the other and the surrounding tiles will become larger as one would expect. However, there exists a special case in which the adjacent tiles in all four directions prevent each other from closing the gap created in a circular fashion. In this situation, the surrounding tiles are re-arranged form a grid split down the center of the deleted tile, such that: tiles furthest to each corner of the grid make up the corresponding quadrant.

### Examples
For now, a few examples can be seen in the `test` directory. Screenshots of the specific cases discussed above will be included here at a later date.

## Features (Planned + Implemented)
### Initial MVP
- Multi-workspace support.
- Gaps between tiles: Non-essential, but entirely trivial to support.
- Axis-aligned resizing along a single row/column.
- Automatic/dynamic AABB selection for manipulation actions. 
- Free-form user selection of the AABB for manipulation actions: Supplied in position+size format, subject to automatic correction.
- Tag-based element lookup: Tags are printable, parsable descriptors for referring to graph elements.
  - Tiles have internal IDs but can also be assigned names their internal ID for ease-of-use.
  - Elements other than tiles themselves are described relative to tiles. E.g. Vertices are tile corners, and Edges are pairs of Vertices.

### Potential Extensions
- Diagonal resizing: Whilst this can be achieved by resizing along each axis individually, the AABB for both axes should be determined simultaneously for the most intuitive results.
- Serialisation: to enable for the creation of predefined and hot-swappable layouts.
- Configurable soft constraints: E.g. minimum/maximum sizes for individual windows. These will be preserved heuristically, where possible, and warnings will be raised when they are not. 
  - Technically, some soft constraints already exist (e.g. gaps are a soft constraint on the distance between windows, windows should each be least 1 unit in size).
  - This extension would allow for additional types of constraints to be specified, and make the response to constraint violations configurable.
- Navigable action history: undo/redo, view history of actions, preview a chain of actions before executing.
  - This could be useful in-case constraints would be violated, or as an editor environment for creating pre-defined layouts
- Coordinate-based element lookup: Will only be implemented if not provided by a suitable framework.
- Window focus tracking + AABB selection mode: To be provided as part of the IPC component, not the core GTM library.

## Out of Scope Ideas
Here are some common concepts and features that, whilst useful to end users, are out of scope for this project due primarily to separation of concerns:
- Tile swapping: Fundamentally unnecessary. You can, and should, use/swap tile names instead.
- Z-layers, tab-stacks, full-screen/monocle mode: these all relate to what is displayed *in* a tile. GTM is only intended to manage *where* tiles are.

# Milestones & Progress
## Completed
- Freely positioned, manually linked workspaces with defined positions (AKA Canvases, in the codebase).
- Algorithms to automatically repair adjacency links after manipulations.
- Printing and parsing of element tags
- Use of tags as lookup keys

## In progress
- Automatic AABB computation
- Single-axis resizing

## Pending to-do list:
- Window deletion
- Formally document tag syntax.
- Interprocess Communication
- Improved examples/documentation.
- Add explanations for all pathological cases.

[awesome]: https://awesomewm.org
[bspwm]: https://github.com/baskerville/bspwm
[i3]: https://i3wm.org