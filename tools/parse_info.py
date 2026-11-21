#!/usr/bin/env python3
"""Parse a `/industrialforegoingsouls info` line into shell assignments.

    Soul Laser Base at [194, 102, 0]: souls 6/1350, progress 3/20, lens industrialforegoing:blue_laser_lens, warden 488.0, t=264
    Soul Pipe at [195, 102, 0]: network 4/4, t=264
    Soul Surge at [194, 102, -1]: network 3/8, wait 290/300, facing south, powered false, t=264

Prints `ok=1` and the fields the line carries (`SOULS=`, `PROGRESS=`, `LENS=`,
`WARDEN=`, `NETWORK=`, `NET_CAP=`, `WAIT=`, `WAIT_MAX=`, `FACING=`, `POWERED=`,
`T=`), or `ok=0`. The e2e `eval`s this.
"""

from __future__ import annotations

import re
import sys

BASE = re.compile(
    r"^Soul Laser Base at \[.*?\]: souls (?P<souls>\d+)/(?P<cap>\d+), "
    r"progress (?P<progress>\d+)/(?P<progress_max>\d+), lens (?P<lens>\S+), "
    r"warden (?P<warden>\S+), t=(?P<t>\d+)$"
)
PIPE = re.compile(
    r"^Soul Pipe at \[.*?\]: network (?P<network>\d+)/(?P<net_cap>\d+), t=(?P<t>\d+)$"
)
SURGE = re.compile(
    r"^Soul Surge at \[.*?\]: network (?P<network>\d+)/(?P<net_cap>\d+), "
    r"wait (?P<wait>\d+)/(?P<wait_max>\d+), facing (?P<facing>\S+), "
    r"powered (?P<powered>true|false), t=(?P<t>\d+)$"
)


def main() -> None:
    line = sys.argv[1] if len(sys.argv) > 1 else ""
    for kind, pattern in (("base", BASE), ("pipe", PIPE), ("surge", SURGE)):
        match = pattern.match(line)
        if not match:
            continue
        print("ok=1")
        print(f"KIND={kind}")
        for name, value in match.groupdict().items():
            print(f"{name.upper()}={value}")
        return
    print("ok=0")


if __name__ == "__main__":
    main()
