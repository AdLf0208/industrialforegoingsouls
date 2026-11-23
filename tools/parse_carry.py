#!/usr/bin/env python3
"""Parse the `/data get entity @s Inventory` reply for a broken machine's carried payload.

The engine answers vanilla's line for everything the player carries:

    Entity <uuid> has the following entity data: [{Slot:0b,count:1,
    id:"industrialforegoingsouls:soul_laser_base",components:
    {"minecraft:block_entity_data":{id:"industrialforegoingsouls:soul_laser_base",
    souls:30,progress:4,bar:16,lens:"industrialforegoing:blue_laser_lens",
    augments:[{Efficiency:0.8d},{}],augment_item:"industrialforegoing:efficiency_addon_tier_2"}}}]

This finds the payload that names one of this mod's machines and prints the shell assignments the
e2e evaluates:

    ok=1 id=industrialforegoingsouls:soul_laser_base souls=30 progress=4 bar=16
    lens=industrialforegoing:blue_laser_lens efficiency=0.8 wait=none

Usage:

    parse_carry.py "<reply line>" none          # no payload at all (an empty machine's item)
    parse_carry.py "<reply line>" <machine id> [souls] [bar] [lens] [efficiency] [wait]

Exit 1 when the reply is not what was asked for.
"""

from __future__ import annotations

import re
import sys

COMPONENT = "minecraft:block_entity_data"
MACHINES = (
    "industrialforegoingsouls:soul_laser_base",
    "industrialforegoingsouls:soul_network_pipe",
    "industrialforegoingsouls:soul_surge",
)


def payload_bodies(line: str):
    """Every `minecraft:block_entity_data` compound on the line, as text without its braces."""
    for match in re.finditer(re.escape(COMPONENT) + r'"?\s*:\s*\{', line):
        start = match.end() - 1
        depth = 0
        quoted = False
        escaped = False
        for index in range(start, len(line)):
            char = line[index]
            if quoted:
                if escaped:
                    escaped = False
                elif char == "\\":
                    escaped = True
                elif char == '"':
                    quoted = False
                continue
            if char == '"':
                quoted = True
            elif char == "{":
                depth += 1
            elif char == "}":
                depth -= 1
                if depth == 0:
                    yield line[start + 1 : index]
                    break


def number(text: str, name: str):
    match = re.search(r"(?<![A-Za-z_])" + name + r"\s*:\s*(-?\d+)", text)
    return int(match.group(1)) if match else None


def word(text: str, name: str):
    match = re.search(r'(?<![A-Za-z_])' + name + r'\s*:\s*"([^"]+)"', text)
    return match.group(1) if match else None


def decimal(text: str, name: str):
    """A number written as a string, or as SNBT's typed form (`0.8d`, `0.8f`, `0.8`)."""
    quoted = word(text, name)
    if quoted is not None:
        return quoted
    match = re.search(r"(?<![A-Za-z_])" + name + r"\s*:\s*(-?[\d.]+)[dfDF]?", text)
    return match.group(1) if match else None


def main(argv: list[str]) -> int:
    if len(argv) < 3:
        print("usage: parse_carry.py '<data get reply>' none|<machine id> [souls] [bar] [lens] [efficiency] [wait]",
              file=sys.stderr)
        return 2
    line, expect = argv[1], argv[2].strip()
    body = None
    for candidate in payload_bodies(line):
        if word(candidate, "id") in MACHINES:
            body = candidate
            break

    if expect == "none":
        ok = body is None
        print(f"ok={1 if ok else 0}")
        print(f"component={'absent' if ok else 'present'}")
        if body is not None:
            print(f"id={word(body, 'id')}")
        return 0 if ok else 1

    if body is None:
        print("ok=0")
        print("component=absent")
        return 1
    found = {
        "id": word(body, "id"),
        "souls": number(body, "souls"),
        "progress": number(body, "progress"),
        "bar": number(body, "bar"),
        "lens": word(body, "lens"),
        "efficiency": decimal(body, "Efficiency"),
        "wait": number(body, "wait"),
    }
    wanted = {
        "id": expect,
        "souls": int(argv[3]) if len(argv) > 3 and argv[3] not in ("", "-") else None,
        "bar": int(argv[4]) if len(argv) > 4 and argv[4] not in ("", "-") else None,
        "lens": argv[5] if len(argv) > 5 and argv[5] not in ("", "-") else None,
        "efficiency": argv[6] if len(argv) > 6 and argv[6] not in ("", "-") else None,
        "wait": int(argv[7]) if len(argv) > 7 and argv[7] not in ("", "-") else None,
    }
    ok = all(want is None or found[key] == want for key, want in wanted.items())
    print(f"ok={1 if ok else 0}")
    for key, value in found.items():
        print(f"{key}={value if value is not None else 'none'}")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
