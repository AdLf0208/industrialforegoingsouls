# Validation — `industrialforegoingsouls` 0.2.5

The ladder actually run for the delivered package, cheapest first. Every compile went through
`ports/tools/build-slot.sh` (`CARGO_INCREMENTAL=0`, no `--target-dir`); `test` and the lints
forwarded to the remote runner. `cargo horizon-mod build`, `check` and both `fits` runs ran locally
(`BUILD_LOCAL=1`) because the remote box has no `cargo-horizon-mod`, and the port's format was
re-proven locally after a remote `cargo fmt` formatted the remote copy only. The in-game legs are
launcher builds at the campaign pin **`0a875da91dbb0fdcb7ba8c42c11341aa229af51a`** + mixins
`e59da909…`, of a game carrying **this port and its parent** (`industrialforegoing` 0.2.11 in this
run).

This is the **config-file** build: the jar's twelve settings (`ConfigSoulLaserBase`,
`ConfigSoulSurge`, `IFSoulsMachines`, `IFSoulsClient`) become
`config/industrialforegoingsouls/config.json`, declared with `register_configs`, written with the
jar's defaults when it is missing, read once in the Config phase (`config_loading`) and again when the
file changes (`config_reloading`) — never from a tick — and clamped to the jar's own ranges. The
values are threaded through the pure rules as `runtime::Settings`; nothing is a constant any more.
`SLICE-PLAN-0.2.5.md` is the plan and the jar's table; `notes.md` §1.6 records the design, §4.5 the
two reds and the three tooling lessons, and §7 the gaps (including the screen bar/arrow, which stays
a named gap: `--offscreen` has no pixel to read).

Evidence: `ports/_shared/evidence/industrialforegoingsouls/` and
`ports/_shared/evidence/industrialforegoingsouls-e2e/` (0.2.0 logs keep a `-0.2.0` suffix, 0.2.1 a
`-0.2.1` one; the unsuffixed logs are this run).

| step | command | result | evidence |
|---|---|---|---|
| red (the parser) | `cargo test` with the seven config rules at `todo!()` | **46 passed, 7 failed** (`not yet implemented: slice 0.2.5: parse the config file`) | `industrialforegoingsouls/sdk-test-red-config.log` |
| red (the rules) | `cargo test` with `&Settings` threaded and the bodies still on the constants | **55 passed, 5 failed** (`left: 2` for a 5-soul operation, `left: 8` for a 12-soul network, `left: 299` for a 120-tick soul, `left: 900` for the 500 tank) | `industrialforegoingsouls/sdk-test-red-0.2.5.log` |
| tests | `cargo test --locked --offline` | **60 passed, 0 failed** | `industrialforegoingsouls/sdk-test.log` |
| format | `cargo fmt --check` (locally) | clean (`FMT-CLEAN`) | `industrialforegoingsouls/sdk-clippy-0.2.5.log` |
| lints | `cargo clippy --all-targets -- -D warnings` | `CLIPPY-EXIT:0`, 0 warnings | `industrialforegoingsouls/sdk-clippy-0.2.5.log` |
| content | `python3 tools/assemble_content.py --check` | `CONTENT OK: 57 files` (the slice adds no content) | — |
| package | `rm -f Cargo.lock && cargo horizon-mod build` ×2, the SDK-generated lock stripped from each archive | `CMP-IDENTICAL`, sha256 `e73258cc4aef8ff5e5e4e46d0f78b22b64c70a457dcf93a1932e0878d4474157`, **121,782 B**, **65** entries, `PACKED-LOCK: 0` | `industrialforegoingsouls/sdk-build-0.2.5.log` |
| check | `cargo horizon-mod check industrialforegoingsouls.hmod` | `industrialforegoingsouls 0.2.5 (native-mod, format native-1/0a875da91dbb0fdcb7ba8c42c11341aa229af51a/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)`, **121,782 B**, sha256 `e73258cc…4157`, exit 0 | `industrialforegoingsouls/sdk-check-0.2.5.log` |
| fits (pin) | `cargo horizon-mod fits industrialforegoingsouls.hmod --build 0a875da` | `0a875da  made for`, exit 0 | `industrialforegoingsouls/sdk-fits-0.2.5.log` |
| fits (other) | `… --build 0b14f8a` | `0b14f8a  fits`, exit 0 | `industrialforegoingsouls/sdk-fits-0.2.5.log` |
| package gate | `python3 ports/tools/verify-sdk-package.py industrialforegoingsouls` | **1/1** (60 tests; runtime byte-identical; every packed file is the tree's; no packed `Cargo.lock`) | — |
| launcher build | `tools/e2e-two-mod.sh build` | `Compiling industrialforegoing (1 of 2)`, **`Compiling industrialforegoingsouls (2 of 2)`**, `the build carries: industrialforegoing 0.2.11, industrialforegoingsouls 0.2.5` (`native-0a875da-893f7451dc2f`) | `industrialforegoingsouls/e2e-two-mod-build-0.2.5.log` |
| in-game, green | `tools/e2e.sh` on that build (A–N, D2–D3, R, S, K–L, P–Q, M2, **T**) | **`PASS` + `EXIT:0`** | `industrialforegoingsouls-e2e/e2e-launcher.log` |
| — | the control and the nine selftests ran on the same tree and manifest one `README.md` edit earlier (`native-0a875da-a0802c696f1a`); the README is packed, so the package was rebuilt and the **green** leg re-run on the final build | as tabled below | as tabled below |
| control | the same script on `GAME=$CONTROL_GAME` (same Nightly, no package) | `FAIL: the game refused industrialforegoingsouls:soul_laser_base` → `EXIT:1` | `…/e2e-launcher-control.log` |
| selftest, **config** | `SELFTEST=config` (the tuned file, the default expectation) | `FAIL: the tank's capacity, from the config in force is 500, expected 1350` → `EXIT:1` | `…/e2e-launcher-selftest-config.log` |
| selftest, lens | `SELFTEST=1` (phase B asserts the tank is not empty without a lens) | `FAIL: selftest: the soul tank is 0, expected anything but 0` → `EXIT:1` | `…/e2e-launcher-selftest.log` |
| selftest, screen | `SELFTEST=screen` (phase S asserts the base opens **no** menu) | `FAIL: selftest: the base opened a menu (container), expected none` → `EXIT:1` | `…/e2e-launcher-selftest-screen.log` |
| selftest, augment | `SELFTEST=augment` (phase D3 asserts the plain bar's rate) | `FAIL: selftest: the shortened bar gained 22 souls, expected no more than the plain bar's 16` → `EXIT:1` | `…/e2e-launcher-selftest-augment.log` |
| selftest, ore | `SELFTEST=laser` (phase M2 asserts the output slot is empty) | `FAIL: selftest: the base yielded minecraft:echo_shard x1, expected an empty output slot` → `EXIT:1` | `…/e2e-launcher-selftest-laser.log` |
| selftest, growth | `SELFTEST=grow` (phase G asserts the unaccelerated wheat grew) | `FAIL: selftest: the control wheat's age is 0, expected it above 0` → `EXIT:1` | `…/e2e-launcher-selftest-grow.log` |
| selftest, carry | `SELFTEST=carry` (phase P asserts the restored machine kept nothing) | `FAIL: selftest: the restored tank is 30, expected 0` → `EXIT:1` | `…/e2e-launcher-selftest-carry.log` |
| selftest, replace | `SELFTEST=replace` (phase Q asserts the second lens voided the first) | `FAIL: selftest: the lens the swap returned is 1, expected 0` → `EXIT:1` | `…/e2e-launcher-selftest-replace.log` |
| selftest, waterlog | `SELFTEST=waterlog` (phase R asserts the bucket left the pipe dry) | `FAIL: selftest: the pipe's waterlogged state is true, expected false` → `EXIT:1` | `…/e2e-launcher-selftest-waterlog.log` |
| log gate | `python3 ports/tools/check-e2e-logs.py industrialforegoingsouls-e2e --strict` | `logs=18 ok=18 missing-exit=0 bad-exit=0` | — |
| skills gate | `python3 ports/tools/check-skills.py --notes ports/industrialforegoingsouls` | PASS | — |

Artifact: **121,782 B**, SHA-256
`e73258cc4aef8ff5e5e4e46d0f78b22b64c70a457dcf93a1932e0878d4474157`, 65 entries, no packed
`Cargo.lock`, declared version **0.2.5**.

## What each in-game phase asserts

The phases 0.2.0–0.2.4 established are unchanged, and this slice adds **T**. In full:

| phase | assertion |
|---|---|
| launch | 0 `load-items`/`recipes`/`advancements`/`content` errors in the game's log |
| A | the three block items register and read back; an unknown item and block id are refused by the game's own message; the machine and a `NoAI` warden read back through `/industrialforegoingsouls info` |
| B | 60 ticks, a warden in range, **no lens** → `souls 0`, `warden 500.0` |
| C | seat the blue lens by right-click (one taken from the hand); 60 ticks → `4 operations`, `8 souls` = `2 × ((500 − health) / 4)`, `warden 484.0` |
| D | the lens comes out through the menu's catalyst slot (shift-click on 300) → `lens none`, the lens in the inventory; 60 more ticks → the tank does not move |
| D2 | right-click with `efficiency_addon_tier_2` → `augments efficiency 0.80` and the addon gone from the hand; one operation later the bar reads `x/16`; a speed addon's click leaves it `held 1` and the efficiency unchanged (and opens the menu, which the phase closes) |
| D3 | the lens back in: over a 160-tick window the 16-tick bar stores 20+ souls, where the plain bar fits eight operations (16) |
| E | the lens out through the menu; `souls 50` seeded; one pipe → the machine at 46 and the network 4/4; a second pipe → 8/8 and the machine at 42 |
| F | a surge placed on a full block's top face takes `facing=up`, and the readout agrees |
| G | `random_tick_speed 0`, a surge on the wheat's north face with one pipe: the network 4 → 3 → 2, `wait` 293 → 0 over 1,200 ticks, the accelerated wheat at **age 7** and the control at **age 0** |
| H | a redstone block beside the surge → `powered true` and `wait` frozen; removed → `false` and falling again |
| R | **waterlogging**: a fresh pipe reads `waterlogged=false` and `0/4`; a water-bucket click reads `true`; the waterlogged pipe pulls the seeded laser base `50 → 46` into `4/4`; an empty-bucket click reads `false` and puts the water bucket in the inventory; a surge at `[facing=north,waterlogged=true]` reads both properties back and spends a soul; a dry surge reads `false`; the laser base refuses the property by the game's own parser; `SELFTEST=waterlog` fails on `true` |
| S | **the menu**: a plain right-click opens it, `ctl screen` lists **3** container slots (301 holds D2's tier-2 addon); the lens goes in through slot 300 and the machine harvests (`souls 44 → 50`, `warden 496 → 484`); the data slots read `[50, 2, 16]` and agree with the line's tank and bar; a tier-1 efficiency addon clicked into 301 and 302 is refused by both and stays on the cursor; a vanilla barrel beside it opens **27** chest slots; `SELFTEST=screen` fails on the menu opening |
| K | the three recipes craft in a real table (8 pipes), the ingredients are consumed, the grid is empty, and a grid of vanilla stand-ins crafts nothing |
| L | each block breaks in survival with an iron pickaxe and drops itself (peaceful from here, so an Enderman cannot end the run mid-break) |
| P | a base with 30 souls, a lens and the tier-2 addon is broken in survival; **nothing spills**, and the drop's `minecraft:block_entity_data` carries `souls 30`, `progress`, `bar 16` and `carried [{Slot:0b, lens}, {Slot:1b, addon with its `titanium:augments`}]`; placing it back reads `souls 30`, a 16-tick bar, the lens and `augments efficiency 0.80`; an **empty** machine's item has no payload and places as a fresh 20-tick machine; a pipe's item carries its 3 souls and a surge's its 120 ticks |
| Q | a second blue lens on a base with one seated leaves the machine with one lens and the player holding one; `SELFTEST=replace` fails on `held 0` |
| N | `/industrialforegoingsouls info` on a vanilla position answers `<pos> is not an industrialforegoingsouls block.` |
| M2 | the parent's ore laser base in the End: `recipes 11/11`; `recipe none` in the End's own biome; `minecraft:deep_dark` at depth 0 → `industrialforegoingsouls:laser_drill_ore/echo_shard`, `weight 1`; one completed bar for the drill's `10000 → 9000 FE` puts **`minecraft:echo_shard x1`** in the output |
| **T** | **the config file**: the game wrote `config/industrialforegoingsouls/config.json` with all twelve keys and the jar's defaults (`soul_storage 1350`, `souls_per_operation 2`); the file is then replaced — **before** a second launch — by `{"soul_storage": 500, "souls_per_operation": 5}`, and the same machine reads `souls 0/500`, clamps a seeded 900 to `500/500`, keeps the unnamed `max_progress` at 20 in the bar and in the menu's data slot, and stores **5** souls in the first operation; `SELFTEST=config` asserts 1,350/2 on that tuned file and fails |

## Notes on the run

- **The dependency is declared.** `horizon-mod.toml` carries
  `industrialforegoing = { version = ">=0.2.7", … }`; a one-module bundle is refused by name before it
  starts (`sdk-dependency-refusal.log`, 0.2.3). The two-module bundle builds both, parent first.
- **`fits` is a two-build check, and it caught a name.** `--build 0a875da` is the pin the package
  declares (`made for`). `--build 0b14f8a` is a different Nightly; the first attempt answered
  `does not fit: 1 item changed — the type Config can no longer be built with a literal, and
  src/runtime.rs:86 builds it`, which is the scanner attributing this port's own `Config` literal to
  `mod_api::Config` (non-constructible). The runtime type is `Settings` now and the line is `fits`;
  no ported behaviour changed.
- **No packed `Cargo.lock`.** `cargo horizon-mod build` writes a probe lock once the manifest declares
  a dependency; the package's own `Cargo.toml` has only the SDK **path** dependency, so the lock is
  not a deliverable and each archive is stripped (`zip -q -d … Cargo.lock`).
- **Toolchain.** `cargo horizon-mod` at this pin needs rust **1.99.1** (`cargo +1.99.1 horizon-mod …`).
- **The config file is the instance's, not `$HORIZON_HOME`'s.** The e2e launches from the scratch root,
  so the game's folder is that root: the template landed at `$SCRATCH/config/industrialforegoingsouls/`
  (measured; a file written under `$HORIZON_HOME/config/` is never read).
- **A phase that summons a monster must not inherit `peaceful`.** Phase M2 sets it, and the first
  tuned run failed with `warden none` and the server's own `Monsters cannot be summoned in Peaceful
  difficulty`; phase T sets the difficulty back and asserts the warden is in the box.
- **`ctl` does not expose a menu's data slots**, so phase S and phase T assert them through
  `/industrialforegoingsouls info`'s `menu [...]` field (the block entity's `data`) — see `notes.md`
  §7. The screen's *drawing* (a tank bar and a progress arrow drawn as geometry) is not asserted and
  is a named, unqueued gap: with `--offscreen` there is no pixel to read.
- Every leg ran in a build-slot lease; `HORIZON_E2E_ROOT` and `E2E_TMP` are under
  `~/.cache/horizon-e2e`, never `/tmp`. The scenario uses `--offscreen`; native commands go through
  `ctl chat`.
