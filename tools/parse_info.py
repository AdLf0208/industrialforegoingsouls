#!/usr/bin/env python3
"""Parse a `/industrialforegoingsouls info` line into shell assignments.

    Soul Laser Base at [194, 102, 0]: souls 6/1350, progress 3/20, lens industrialforegoing:blue_laser_lens, augments efficiency 0.80, menu [6, 3, 20], warden 488.0, t=264
    Soul Pipe at [195, 102, 0]: network 4/4, waterlogged false, t=264
    Soul Surge at [194, 102, -1]: network 3/8, wait 290/300, facing south, waterlogged false, powered false, t=264

Prints `ok=1` and the fields the line carries (`SOULS=`, `PROGRESS=`,
`PROGRESS_MAX=`, `LENS=`, `EFFICIENCY=`, `MENU_SOULS=`, `MENU_PROGRESS=`,
`MENU_BAR=`, `WARDEN=`, `NETWORK=`, `NET_CAP=`, `WATERLOGGED=`, `WAIT=`,
`WAIT_MAX=`, `FACING=`, `POWERED=`, `T=`), or `ok=0`. `MENU_*` are the
block entity's menu data slots — the values the menu's screen draws — in
order: the tank, the bar's progress and the bar's length. The e2e `eval`s
this.
"""

from __future__ import annotations

import re
import sys

BASE = re.compile(
    r"^Soul Laser Base at \[.*?\]: souls (?P<souls>\d+)/(?P<cap>\d+), "
    r"progress (?P<progress>\d+)/(?P<progress_max>\d+), lens (?P<lens>\S+), "
    r"augments efficiency (?P<efficiency>[\d.]+), menu \[(?P<menu>[^\]]*)\], "
    r"warden (?P<warden>\S+), t=(?P<t>\d+)$"
)
PIPE = re.compile(
    r"^Soul Pipe at \[.*?\]: network (?P<network>\d+)/(?P<net_cap>\d+), "
    r"waterlogged (?P<waterlogged>true|false), t=(?P<t>\d+)$"
)
SURGE = re.compile(
    r"^Soul Surge at \[.*?\]: network (?P<network>\d+)/(?P<net_cap>\d+), "
    r"wait (?P<wait>\d+)/(?P<wait_max>\d+), facing (?P<facing>\S+), "
    r"waterlogged (?P<waterlogged>true|false), "
    r"powered (?P<powered>true|false), t=(?P<t>\d+)$"
)

MENU_FIELDS = ("menu_souls", "menu_progress", "menu_bar")


def main() -> None:
    line = sys.argv[1] if len(sys.argv) > 1 else ""
    for kind, pattern in (("base", BASE), ("pipe", PIPE), ("surge", SURGE)):
        match = pattern.match(line)
        if not match:
            continue
        print("ok=1")
        print(f"KIND={kind}")
        fields = match.groupdict()
        menu = fields.pop("menu", None)
        for name, value in fields.items():
            print(f"{name.upper()}={value}")
        if menu is not None:
            values = [value.strip() for value in menu.split(",")]
            if len(values) != len(MENU_FIELDS) or not all(
                value.lstrip("-").isdigit() for value in values
            ):
                print("ok=0")
                return
            for name, value in zip(MENU_FIELDS, values):
                print(f"{name.upper()}={value}")
        return
    print("ok=0")


if __name__ == "__main__":
    main()
