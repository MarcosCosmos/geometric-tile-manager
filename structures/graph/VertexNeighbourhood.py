from structures.geomtry import CardinalDataclass
from structures.graph import Vertex
from typing import MutableSequence

from utility import enum_dataclass


@enum_dataclass
class VertexNeighbourhood(CardinalDataclass[MutableSequence['Vertex']]):
    """
    Slightly different from ordinary definitions, neighbourhoods here are directionally subdivided as they must be axis aligned, ordered, and we need to know the neighbours in both directions if a point lies between two neighbours along an axis.
    Neighbourhoods, unlike boxes, are often subject to updates and do not need to be treated as immutable.


    Neighbourhoods are initialised to be empty on all sides, then become populated after the vertex is created, due to the need for circular reference.

    Note: After proper population, the neighbours may be ONLY be None for the outgoing directions of a Wall.

    Note: the possible lengths on each side for vertex neighbours must be in the range [0,2], where 0 is only possible for the outward sides of a Wall and 2 is only possible if the vertex is between the corners of two corners of the associated neighbouring tile(s)
    """