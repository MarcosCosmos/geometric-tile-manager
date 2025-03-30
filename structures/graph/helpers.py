from __future__ import annotations

from typing import Sequence
import parse as ps


def parse_tag(format: str, text: str) -> Sequence[str]:
    result = ps.parse(format, text)
    return [result.named[each[0]] for each in sorted(result.spans.items(), key=lambda x: x[1])]
