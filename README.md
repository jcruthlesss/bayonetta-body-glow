# Bayonetta c02 body animation glow

Smashline 2 runtime controller for the `body_anim` to `body_normal` transition.

## Behavior

- Runs only for Bayonetta costume slot `c02`.
- Detects `body_anim` directly, independent of the current motion or status.
- Plays one immediate opening flash with a four-frame falloff.
- Detects the game's actual request to return to `body_normal`.
- Holds `body_anim` for eight additional frames under an extreme white-purple
  flash and `sys_aura_light`, then completes the normal model switch.
- Clears its state and effects on death, rebirth, or entry.

## Remove the earlier motion tests

Do not use the modified `.nuanmb` files from glow test V1 or V2 with this
plugin. Restore your original motion files first. The plugin replaces their
timing-based glow behavior.

## Build

This project requires a working Skyline Rust development environment with the
`aarch64-skyline-switch` target and Smashline 2 dependencies.

From the project directory:

```sh
cargo skyline build --release
```

The resulting NRO should be produced under the release target directory. If
your environment uses the older cargo setup, use `cargo build --release`.

## Install

Copy the compiled `.nro` to:

```text
atmosphere/contents/01006A800016E000/romfs/skyline/plugins/
```

Keep Skyline and Smashline 2 installed. Reboot the game after replacing the
plugin.

## Tuning

The main timing constant is near the top of `src/lib.rs`:

```rust
const END_HOLD_FRAMES: i32 = 8;
```

Increase it if the ending glow should last longer. Decrease it if the held
transformed pose is too noticeable.

The opening and ending brightness values are in `opening_flash` and
`ending_flash` respectively.

## Test carefully

This is a first runtime test and has not been executed on hardware in this
workspace. Test offline first. If it fails to compile against a particular
dependency revision, retain the full compiler output so the exact API mismatch
can be corrected without guessing.
