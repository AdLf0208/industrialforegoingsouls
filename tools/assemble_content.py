#!/usr/bin/env python3
"""Assemble the port's `assets/` and `data/` from the pack's Industrial Foregoing: Souls jar.

The jar is the ATM10 file (`ports/_shared/licence-cache/industrial-foregoing-souls-1.21.1-1.10.7.jar`,
sha256 `f4fb3640…67ec`), kept as the licence and provenance evidence. The blockstates, models, block
textures, loot tables and recipes come out of the jar, re-spelled only where 26.3 differs:

* `{"tag": T}` / `{"item": I}` ingredients -> `"#T"` / `"I"`;
* a loot pool's `conditions` list -> one `condition` with a typed `type` (the 1.21.1 plural form
  evaluates false at 26.3 and the block drops nothing);
* the unknown `neoforge:conditions` gate dropped (the item exists whenever this mod does);
* the pipe's three-value connection enum (`none|block|pipe`) -> the engine's `chorus_plant` template
  with a boolean a side, because 26.3 builds a pipe's shape and joins itself (see notes.md);
* the surge's `enabled`/connection properties are **not** declared: this pin cannot rewrite a block
  state outside `set_block`, so the surge draws as its model and its redstone rule lives in the tick;
* the one `laser_drill_ore` recipe (echo shard, deep dark) is carried as the jar writes it: its type
  is the parent package's, and its object-form `catalyst`/`output` are that type's own serializer's
  dialect, not a shaped recipe's ingredient.

The block/item declarations, the 26.3 item definitions and the lang subset are written here. Rerun it
to rebuild (`assets/` and `data/` are deleted first):

    python3 tools/assemble_content.py            # write
    python3 tools/assemble_content.py --check     # exit 1 if the tree is not what this would write
"""

from __future__ import annotations

import hashlib
import json
import shutil
import sys
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
JAR = ROOT.parent / "_shared/licence-cache/industrial-foregoing-souls-1.21.1-1.10.7.jar"
JAR_SHA256 = "f4fb3640fdfca313688f2e0561b1875cbf740271dbcf5a276c4d67347d7b67ec"
MOD = "industrialforegoingsouls"

BLOCKS = ["soul_laser_base", "soul_network_pipe", "soul_surge"]

# The blocks one model file each. The pipe's side models come from the jar too; the surge keeps its
# one model (its `off` and connection pieces belong to properties this port does not declare).
BLOCK_MODELS = [
    "block/soul_laser_base",
    "block/soul_network_pipe_middle",
    "block/soul_network_pipe_side",
    "block/soul_network_pipe_item",
    "block/soul_surge",
]

# `c:stones` in 26.3: the convention tag exists in no vanilla pack, so the port declares the 12
# vanilla natural stones the campaign's other ports declare (simplylight's list, so every port
# shipping the tag agrees). Its only consumer here is two recipes; the e2e crafts with
# `minecraft:stone`, which every variant of the tag holds.
STONES = [
    "minecraft:stone",
    "minecraft:granite",
    "minecraft:diorite",
    "minecraft:andesite",
    "minecraft:deepslate",
    "minecraft:tuff",
    "minecraft:calcite",
    "minecraft:basalt",
    "minecraft:smooth_basalt",
    "minecraft:blackstone",
    "minecraft:dripstone_block",
    "minecraft:netherrack",
]

LANG = {
    "block.industrialforegoingsouls.soul_laser_base": "Soul Laser Base",
    "block.industrialforegoingsouls.soul_network_pipe": "Soul Pipe",
    "block.industrialforegoingsouls.soul_surge": "Soul Surge",
    "industrialforegoingsouls.soul_storage": "Soul Storage: ",
    "itemGroup.industrialforegoingsouls": "Industrial Foregoing: Souls",
}


def jar() -> zipfile.ZipFile:
    data = JAR.read_bytes()
    got = hashlib.sha256(data).hexdigest()
    if got != JAR_SHA256:
        raise SystemExit(f"{JAR.name}: sha256 {got} != recorded {JAR_SHA256}")
    return zipfile.ZipFile(JAR)


def read(archive: zipfile.ZipFile, name: str) -> bytes:
    return archive.read(f"assets/{MOD}/{name}")


def read_data(archive: zipfile.ZipFile, name: str) -> bytes:
    return archive.read(f"data/{MOD}/{name}")


def model(textures: dict[str, str], elements: list, parent: str | None = None) -> dict:
    out: dict = {}
    if parent:
        out["parent"] = parent
    out["textures"] = textures
    out["elements"] = elements
    return out


def build() -> dict[str, bytes]:
    """Every file the content tree is, as `path -> bytes`."""
    archive = jar()
    files: dict[str, bytes] = {}

    def put(path: str, value: str | bytes | dict) -> None:
        if isinstance(value, dict):
            files[path] = (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()
        elif isinstance(value, str):
            files[path] = value.encode()
        else:
            files[path] = value

    # --- declarations -------------------------------------------------------------------------
    iron = {
        "explosion_resistance": 6.0,
        "hardness": 5.0,
        "requires_tool": True,
        "sounds": "minecraft:iron_block",
    }
    put(
        f"data/{MOD}/horizon/block/soul_laser_base.json",
        {"item": True, "properties": iron, "template": "minecraft:iron_block"},
    )
    put(
        f"data/{MOD}/horizon/block/soul_network_pipe.json",
        {
            "connects_to": f"#{MOD}:soul_connects",
            "item": True,
            "properties": iron,
            "template": "minecraft:chorus_plant",
            "thickness": 6,
        },
    )
    put(
        f"data/{MOD}/horizon/block/soul_surge.json",
        {
            "default_state": {"facing": "north"},
            "item": True,
            "properties": iron,
            "state_properties": {
                "facing": {
                    "type": "enum",
                    "values": ["north", "south", "east", "west", "up", "down"],
                }
            },
            "template": "minecraft:iron_block",
        },
    )
    put(
        f"data/{MOD}/horizon/creative_tab/souls.json",
        {
            "icon": f"{MOD}:soul_laser_base",
            "items": [f"{MOD}:{name}" for name in BLOCKS],
            "title": "itemGroup.industrialforegoingsouls",
        },
    )

    # --- assets: models, blockstates, item definitions, textures, lang ------------------------
    for name in BLOCK_MODELS:
        put(f"assets/{MOD}/models/{name}.json", read(archive, f"models/{name}.json"))

    put(
        f"assets/{MOD}/blockstates/soul_laser_base.json",
        {"variants": {"": {"model": f"{MOD}:block/soul_laser_base"}}},
    )
    # The jar's three-value enum collapses to the engine's one boolean a side.
    sides = [
        ("north", {}),
        ("south", {"y": 180}),
        ("east", {"y": 90}),
        ("west", {"y": 270}),
        ("down", {"x": 90}),
        ("up", {"x": 270}),
    ]
    put(
        f"assets/{MOD}/blockstates/soul_network_pipe.json",
        {
            "multipart": [
                {"apply": {"model": f"{MOD}:block/soul_network_pipe_middle"}},
                *[
                    {
                        "apply": {"model": f"{MOD}:block/soul_network_pipe_side", **turn},
                        "when": {side: "true"},
                    }
                    for side, turn in sides
                ],
            ]
        },
    )
    # The surge's own six-way rotation, from the jar's `enabled=true` variants; the `off` and
    # connection pieces belong to properties this port does not declare.
    turns = {
        "down": {"x": 90},
        "up": {"x": -90},
        "north": {},
        "south": {"y": -180},
        "west": {"y": -90},
        "east": {"y": 90},
    }
    put(
        f"assets/{MOD}/blockstates/soul_surge.json",
        {
            "variants": {
                f"facing={facing}": {"model": f"{MOD}:block/soul_surge", **turn}
                for facing, turn in turns.items()
            }
        },
    )

    for name in BLOCKS:
        put(
            f"assets/{MOD}/items/{name}.json",
            {"model": {"model": f"{MOD}:item/{name}", "type": "minecraft:model"}},
        )
    put(
        f"assets/{MOD}/models/item/soul_laser_base.json",
        {"parent": f"{MOD}:block/soul_laser_base"},
    )
    put(
        f"assets/{MOD}/models/item/soul_network_pipe.json",
        {"parent": f"{MOD}:block/soul_network_pipe_item"},
    )
    put(f"assets/{MOD}/models/item/soul_surge.json", {"parent": f"{MOD}:block/soul_surge"})

    # The jar's block textures and their `.mcmeta` animations; the GUI texture belongs to the
    # screen this port does not carry.
    for entry in archive.namelist():
        if entry.startswith(f"assets/{MOD}/textures/block/") and not entry.endswith("/"):
            files[f"assets/{MOD}/textures/{entry[len(f'assets/{MOD}/textures/'):]}"] = archive.read(
                entry
            )
    put(f"assets/{MOD}/lang/en_us.json", json.dumps(LANG, indent=2, sort_keys=True) + "\n")

    # --- data: loot, recipes, tags ------------------------------------------------------------
    for name in BLOCKS:
        loot = json.loads(read_data(archive, f"loot_table/blocks/{name}.json"))
        for pool in loot["pools"]:
            conditions = pool.pop("conditions", [])
            if len(conditions) == 1:
                pool["condition"] = {"type": conditions[0]["condition"]}
            elif conditions:
                raise SystemExit(f"{name}: a loot pool with {len(conditions)} conditions")
        put(f"data/{MOD}/loot_table/blocks/{name}.json", loot)

    for name in BLOCKS:
        recipe = json.loads(read_data(archive, f"recipe/{name}.json"))
        recipe.pop("neoforge:conditions", None)
        recipe["key"] = {
            key: f"#{value['tag']}" if "tag" in value else value["item"]
            for key, value in recipe["key"].items()
        }
        put(f"data/{MOD}/recipe/{name}.json", recipe)

    put(
        f"data/{MOD}/tags/block/soul_connects.json",
        {"values": [f"{MOD}:soul_network_pipe", f"{MOD}:soul_surge"]},
    )
    put(f"data/{MOD}/tags/block/cant_accelerate.json", {"values": [f"{MOD}:soul_surge"]})
    put(f"data/c/tags/block/tick_acceleration_disallowed.json", {"values": [f"{MOD}:soul_surge"]})
    put("data/c/tags/item/stones.json", {"values": STONES})
    put(
        "data/minecraft/tags/block/mineable/pickaxe.json",
        {"values": [f"{MOD}:{name}" for name in BLOCKS]},
    )

    # --- the recipe advancements (the jar's own, re-spelled) ---------------------------------
    for name in BLOCKS:
        entry = f"data/{MOD}/advancement/recipes/misc/{name}.json"
        advancement = json.loads(read_data(archive, f"advancement/recipes/misc/{name}.json"))
        advancement.pop("neoforge:conditions", None)
        put(entry, advancement)

    # --- the parent's ore recipe: the one `laser_drill_ore` file the jar carries ---------------
    # The recipe type is `industrialforegoing:laser_drill_ore`, which the **parent** package
    # registers; a game without the parent refuses this package at load anyway (the lens is the
    # parent's item), so the file always has its type. The jar's folder holds one file (echo
    # shard); its `deep_dark` biome tag is what the rarity's whitelist names.
    put(
        f"data/{MOD}/recipe/laser_drill_ore/echo_shard.json",
        json.loads(read_data(archive, "recipe/laser_drill_ore/echo_shard.json")),
    )
    put(
        f"data/{MOD}/tags/worldgen/biome/deep_dark.json",
        {"values": ["minecraft:deep_dark"]},
    )
    put(
        f"data/{MOD}/advancement/laser_drill_ore/echo_shard.json",
        json.loads(read_data(archive, "advancement/laser_drill_ore/echo_shard.json")),
    )

    return files


def main() -> None:
    files = build()
    check = "--check" in sys.argv[1:]
    if check:
        stale = [
            path
            for path, body in files.items()
            if not (ROOT / path).is_file() or (ROOT / path).read_bytes() != body
        ]
        extra = [
            str(path.relative_to(ROOT))
            for top in ("assets", "data")
            for path in (ROOT / top).rglob("*")
            if path.is_file() and str(path.relative_to(ROOT)) not in files
        ]
        if stale or extra:
            for path in stale:
                print(f"differs or missing: {path}", file=sys.stderr)
            for path in extra:
                print(f"unexpected file: {path}", file=sys.stderr)
            raise SystemExit(1)
        print(f"CONTENT OK: {len(files)} files")
        return
    for top in ("assets", "data"):
        shutil.rmtree(ROOT / top, ignore_errors=True)
    for path, body in sorted(files.items()):
        target = ROOT / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(body)
    print(f"wrote {len(files)} files")


if __name__ == "__main__":
    main()
