#!/usr/bin/env python3
"""Craft every shaped recipe this port carries, in one crafting table.

Called by `tools/e2e.sh` with the open game's control socket and a placed crafting
table's position. Each case puts its declared grid into the table, asserts the exact
result id and count, takes it, and asserts the inventory holds the result and none of
the ingredients; the control case puts vanilla stand-ins into the laser base's grid and
asserts the result slot stays empty, which is the counterfactual the recipes turn over.

The carried recipes, from the jar (re-spelled to 26.3, see tools/assemble_content.py):

    soul_laser_base  ABA/CDC/GSG  A=#c:plastics  B=sculk_shrieker  C=iron_bars
                                  D=#industrialforegoing:machine_frame/advanced
                                  G=#c:gears/diamond  S=sculk_catalyst
    soul_network_pipe SSS/CDC/SSS S=#c:stones  C=#c:plastics  D=echo_shard      -> 8
    soul_surge        ABA/CBC/CPC  A=#c:stones  B=#c:plastics  C=echo_shard  P=piston

`#c:plastics`, `#c:gears/diamond` and `#industrialforegoing:machine_frame/advanced` are
the **parent port's** tags (industrialforegoing 0.2.4); `#c:stones` is this port's own
declaration, because 26.3 ships no `c:` item tags.

The harness's crafting table menu numbers its grid 100-108 row-major and its result slot
200 (`horizon-ctl`'s `open-inventory` documents the same layout for the player's 2x2).
"""
import json
import subprocess
import sys

GRID = [str(slot) for slot in range(100, 109)]
RESULT = "200"

BASE = "industrialforegoingsouls:soul_laser_base"
PIPE = "industrialforegoingsouls:soul_network_pipe"
SURGE = "industrialforegoingsouls:soul_surge"
PLASTIC = "industrialforegoing:plastic"
FRAME = "industrialforegoing:machine_frame_advanced"
GEAR = "industrialforegoing:diamond_gear"
STONE = "minecraft:stone"

# (label, expected result id, expected count or None for the control, ingredients)
CASES = [
    (
        "the soul laser base from the parent's tags",
        BASE,
        1,
        [
            (PLASTIC, 2, ["100", "102"]),
            ("minecraft:sculk_shrieker", 1, ["101"]),
            ("minecraft:iron_bars", 2, ["103", "105"]),
            (FRAME, 1, ["104"]),
            (GEAR, 2, ["106", "108"]),
            ("minecraft:sculk_catalyst", 1, ["107"]),
        ],
    ),
    (
        "the control: vanilla stand-ins for the tags leave the result empty",
        None,
        None,
        [
            ("minecraft:iron_block", 5, ["100", "102", "104", "106", "108"]),
            ("minecraft:sculk_shrieker", 1, ["101"]),
            ("minecraft:iron_bars", 2, ["103", "105"]),
            ("minecraft:sculk_catalyst", 1, ["107"]),
        ],
    ),
    (
        "the soul pipe, eight from six #c:stones",
        PIPE,
        8,
        [
            (STONE, 6, ["100", "101", "102", "106", "107", "108"]),
            (PLASTIC, 2, ["103", "105"]),
            ("minecraft:echo_shard", 1, ["104"]),
        ],
    ),
    (
        "the soul surge",
        SURGE,
        1,
        [
            (STONE, 2, ["100", "102"]),
            (PLASTIC, 2, ["101", "104"]),
            ("minecraft:echo_shard", 4, ["103", "105", "106", "108"]),
            ("minecraft:piston", 1, ["107"]),
        ],
    ),
]


def main():
    sock, ctl, x, y, z = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5]

    def run(*args):
        print("+ ctl", *args, file=sys.stderr, flush=True)
        try:
            out = subprocess.run(
                [ctl, "--socket", sock, "--timeout", "90", *args],
                text=True,
                capture_output=True,
                timeout=120,
            )
        except subprocess.TimeoutExpired:
            raise SystemExit(f"ctl {' '.join(args)}: no reply in 120 s")
        try:
            return json.loads(out.stdout)
        except json.JSONDecodeError:
            raise SystemExit(f"ctl {' '.join(args)}: {out.stdout}{out.stderr}")

    def wait(ticks=4):
        run("wait", str(ticks))

    def slots():
        screen = run("screen")
        if "slots" not in screen:
            raise SystemExit(f"no menu is open: {screen}")
        return screen["slots"]

    def find(item):
        for slot in slots():
            if slot.get("item") == item and slot["slot"] < 100:
                return slot["slot"]
        occupied = [(s["slot"], s.get("item"), s.get("count")) for s in slots() if s.get("item")]
        got = run("inventory")["counts"]
        raise SystemExit(
            f"missing ingredient {item} in the inventory: screen {occupied}, counts {got}"
        )

    def open_table():
        run("look-at-block", x, y, z)
        wait()
        print("  crosshair ->", json.dumps(run("looking-at")), file=sys.stderr)
        print("  interact ->", json.dumps(run("interact", x, y, z)), file=sys.stderr)
        wait(6)
        screen = run("screen")
        if "slots" not in screen:
            raise SystemExit(f"the crafting table at {x},{y},{z} did not open: {screen}")

    def result():
        stack = [s for s in slots() if s["slot"] == int(RESULT)]
        if not stack or not stack[0].get("item"):
            return None
        return stack[0]

    def dump(label):
        occupied = [(s["slot"], s.get("item"), s.get("count")) for s in slots() if s.get("item")]
        print(f"  {label}: {occupied}", file=sys.stderr)

    def take_result():
        run("click-slot", RESULT)
        wait(6)

    def place(item, count, cells):
        source = find(item)
        run("click-slot", str(source))
        wait(1)
        if len(cells) == 1:
            run("click-slot", cells[0])
            wait(1)
        else:
            for cell in cells:
                run("click-slot", cell, "right")
                wait(1)
        run("click-slot", str(source))
        wait(2)

    def setup(ingredients):
        try:
            run("close")
        except SystemExit:
            pass
        wait(3)
        run("command", "clear @s")
        for item, count, _ in ingredients:
            run("give", item, str(count))
        wait(6)
        open_table()

    failures = []
    for label, expected, count_expected, ingredients in CASES:
        setup(ingredients)
        for item, count, cells in ingredients:
            place(item, count, cells)
        got = result()
        print(f"{label} -> {json.dumps(got)}")
        if expected is None:
            if got is not None:
                dump(label)
                failures.append(f"{label}: a grid of stand-ins crafted {got['item']}")
            continue
        if not got or got.get("item") != expected or got.get("count") != count_expected:
            dump(label)
            failures.append(f"{label}: expected {count_expected} {expected}, got {got}")
            continue
        take_result()
        counts = run("inventory")["counts"]
        if counts.get(expected, 0) != count_expected:
            failures.append(
                f"{label}: the crafted {expected} did not reach the inventory "
                f"({counts.get(expected)})"
            )
        for item, _, _ in ingredients:
            if counts.get(item, 0) != 0:
                failures.append(f"{label}: {item} was not consumed ({counts.get(item)} left)")
        left = [s for s in slots() if s["slot"] in range(100, 109) and s.get("item")]
        if left:
            failures.append(f"{label}: the grid still holds {left}")

    try:
        run("close")
        wait(3)
    except SystemExit:
        pass
    if failures:
        for failure in failures:
            print(f"FAIL: {failure}", file=sys.stderr)
        raise SystemExit(1)
    print(f"crafted {len([c for c in CASES if c[1]])} shaped recipes, and the stand-in grid stayed empty")


if __name__ == "__main__":
    main()
