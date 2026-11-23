#!/usr/bin/env python3
"""Parse the `/data get entity @s Inventory` reply for a broken machine's carried payload.

The engine answers vanilla's line for everything the player carries:

    Entity <uuid> has the following entity data: [{Slot:0b,count:1,
    id:"industrialforegoingsouls:soul_laser_base",components:
    {"minecraft:block_entity_data":{id:"industrialforegoingsouls:soul_laser_base",
    souls:30,progress:4,bar:16,carried:[{Slot:0b,count:1,
    id:"industrialforegoing:blue_laser_lens"},{Slot:1b,count:1,
    id:"industrialforegoing:efficiency_addon_tier_2",components:
    {"titanium:augments":{Efficiency:0.8d}}}]}}}]

This finds the payload that names one of this mod's machines and prints the shell assignments the
e2e evaluates:

    ok=1 id=industrialforegoingsouls:soul_laser_base souls=30 progress=4 bar=16
    lens=industrialforegoing:blue_laser_lens efficiency=0.8 wait=none items=<ids>

`lens` is the item in the container's slot 0 and `efficiency` the `Efficiency` an augment slot
carries (0.2.4 carries the slots as `carried`); `items` lists every carried id, comma-separated.

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


def machine_id(text: str):
    """The first `id` in `text` that names one of this mod's machines (the payload's own, which is
    not always the first `id`: a `carried` entry before it names an item)."""
    for value in re.findall(r'(?<![A-Za-z_])id\s*:\s*"([^"]+)"', text):
        if value in MACHINES:
            return value
    return None


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


def carried_entries(body: str) -> list[str]:
    """The `carried` list's entries, each a compound's text without its braces."""
    match = re.search(r"(?<![A-Za-z_])carried\s*:\s*\[", body)
    if not match:
        return []
    entries = []
    index = match.end()
    depth = 1
    while index < len(body):
        char = body[index]
        if char == '"':
            index += 1
            while index < len(body):
                if body[index] == "\\":
                    index += 2
                    continue
                if body[index] == '"':
                    break
                index += 1
        elif char == "[":
            depth += 1
        elif char == "]":
            depth -= 1
            if depth == 0:
                break
        elif char == "{" and depth == 1:
            start, inner = index, 0
            while index < len(body):
                if body[index] == "{":
                    inner += 1
                elif body[index] == "}":
                    inner -= 1
                    if inner == 0:
                        break
                index += 1
            entries.append(body[start + 1 : index])
        index += 1
    return entries


def main(argv: list[str]) -> int:
    if len(argv) < 3:
        print("usage: parse_carry.py '<data get reply>' none|<machine id> [souls] [bar] [lens] [efficiency] [wait]",
              file=sys.stderr)
        return 2
    line, expect = argv[1], argv[2].strip()
    body = None
    for candidate in payload_bodies(line):
        if machine_id(candidate) is not None:
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
    entries = carried_entries(body)
    lens = next(
        (word(entry, "id") for entry in entries if number(entry, "Slot") == 0),
        None,
    )
    efficiency = next(
        (decimal(entry, "Efficiency") for entry in entries if decimal(entry, "Efficiency")),
        None,
    )
    found = {
        "id": machine_id(body),
        "souls": number(body, "souls"),
        "progress": number(body, "progress"),
        "bar": number(body, "bar"),
        "lens": lens if lens is not None else word(body, "lens"),
        "efficiency": efficiency if efficiency is not None else decimal(body, "Efficiency"),
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
    ids = ",".join(word(entry, "id") or "" for entry in entries)
    print(f"items={ids if ids else 'none'}")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
