# airbreak-station

https://github.com/user-attachments/assets/e52c815c-e0ff-4401-8013-337ad951e157



`airbreak-station` owns the AirBreak firmware UI workflow end to end:

1. build a small Thumb code-cave payload,
2. patch the stock STM32 firmware with guarded offsets and repaired CRCs,
3. boot the patched firmware in the STM32 emulator.

The older sibling repositories are seed material only:

- `../airbreak-reverse`: historical reverse-engineering notes and failed patch experiments.
- `../airbreak-emulator`: the emulator bring-up that proved the UI can boot.

New patch and emulator work should happen here.

## Firmware

Firmware binaries are private and ignored by git. Put the stock image here:

```bash
mkdir -p firmware
cp /path/to/resmed-air10.bin firmware/resmed-air10.bin
```

or pass it explicitly:

```bash
AIRBREAK_SOURCE_FIRMWARE=/path/to/resmed-air10.bin ./scripts/run-station-pipeline.sh
```

## Run

```bash
./scripts/run-station-pipeline.sh
```

The default pipeline writes:

- patched firmware: `artifacts/firmware/stm32-ui-button.bin`
- patch build products: `artifacts/build/`

By default it opens the interactive SDL emulator GUI. The command keeps running until the emulator
window is closed.

Headless regression check with PNG evidence:

```bash
AIRBREAK_EMULATOR_MODE=headless ./scripts/run-station-pipeline.sh
```

Headless evidence is written under `.airbreak-rust-runs/<run-id>/lcd-frame.png`.
A successful run prints `station_pipeline=pass ... result=pass`.

Static PNG regression checks compare emulator output against committed per-firmware baselines:

```bash
./scripts/run-png-regressions.sh
```

The default selection runs the `air10-vauto` baselines. To refresh baselines after an intentional UI change:

```bash
AIRBREAK_PNG_REGRESSION_MODE=update ./scripts/run-png-regressions.sh
```

Firmware and case manifests live under `tests/png-regressions/`. Cases name UI targets such as `block_breaker`,
`custom_about`, and `clinical_mode`; the runner derives the encoder sequence from the same `AIRBREAK_UI_SCREENS` model used
by the patch pipeline. A case can also override the select delay to exercise fast encoder-press transitions into a screen.
Runtime logs, captured actual PNGs, patched firmware, and diff PNGs are written under
`artifacts/png-regressions/`, which is ignored by git. Use `AIRBREAK_PNG_REGRESSION_FIRMWARES=all` or `firmware-list` to run
the same static PNG cases across the local firmware set.

Patch only, without emulator:

```bash
AIRBREAK_EMULATE=0 ./scripts/run-station-pipeline.sh
```

Custom About text and Clinical Mode labels can be changed at patch time:

```bash
AIRBREAK_CUSTOM_ABOUT_LABEL="Service Notes" \
AIRBREAK_CUSTOM_ABOUT_DETAIL="This is Custom About" \
AIRBREAK_CLINICAL_LABEL="Clinical Mode" \
AIRBREAK_EMULATE=0 ./scripts/run-station-pipeline.sh
```

AirBreak UI screens are selected as a model, not by editing individual hook sites. The runner converts this list into the payload's
`AIRBREAK_UI_SCREEN_MODEL_COUNT` and `AIRBREAK_UI_SCREEN_MODEL_INIT` defines, so adding or removing a screen does not require
row-position edits in the firmware patch. The default model is:

```bash
AIRBREAK_UI_SCREENS=block_breaker,custom_about,clinical_mode
```

To remove Block Breaker from the firmware patch:

```bash
AIRBREAK_ENABLE_BLOCK_BREAKER=0 AIRBREAK_EMULATE=0 ./scripts/run-station-pipeline.sh
```

That builds a smaller payload, derives My Options capacity for two AirBreak rows, and does not patch the Block Breaker page,
LCD render takeover, event gate, post-render tick hook, or Block Breaker text slots.

The patch tool resolves the active `SX567-0401` text layout before writing label pointers. It anchors on firmware strings
such as `My Options`, `Back`, and `View Oximeter`, computes the text-table delta for the model variant, and applies AirBreak
label patches at the resolved slots instead of assuming one fixed model layout.

## Current Patch

The active patch hooks the rendered My Options pages used by both `Essentials=On` and the `Essentials=Plus` expanded view, then appends the rows declared by the AirBreak UI screen model:

- startup check patch: `0x000000F0`, `8442 -> c046`
- compact `Essentials=On` My Options capacity patch: `0x08061792`, `movs r2,#11 -> movs r2,#(11 + enabled AirBreak rows)`
- compact `Essentials=On` My Options final append hook: `0x0806194E`, original target `0x08064E8C`, new target `0x080FF000`
- rendered `Essentials=Plus` My Options capacity patch: `0x0806153E`, `movs r2,#16 -> movs r2,#(16 + enabled AirBreak rows)`
- rendered `Essentials=Plus` My Options final append hook: `0x0806177E`, original target `0x08064E8C`, new target `0x080FF000`
- block breaker row: blank-mapped label id `0xE7` with its English pointer patched to the code-cave `AIRBREAK_BLOCK_BREAKER_LABEL` string
- custom label row: blank-mapped label id `0xE2` with its English pointer patched to the code-cave `AIRBREAK_CUSTOM_ABOUT_LABEL` string
- AirBreak page hosts: page indices `13` and `14` are retitled through dynamic label id `0xE8`; early seed hooks install reusable AirBreak rows, and active Block Breaker exposes zero stock page rows while it owns the LCD frame
- Block Breaker LCD takeover: the menu render entry at `0x08064FBE` is wrapped so active Block Breaker state skips the stock menu renderer and draws a full-frame game surface; the global event setter at `0x08066E7E` is gated so stock UI page, row, and selected-row changes are ignored while the game is active; the post-render wait call at `0x0808DFE6` drains entry-time encoder residue, consumes and clears the rotary provider at `0x200174E4`, falls back to raw PF10/PF11 encoder phase reads for paddle movement, polls the PG11 encoder button for fire, polls PG7 Home as the only game exit, and advances the ball on an internal tick; raw LCD writes use `0x64000000/0x64000002` with `0x2A/0x2B/0x2C`
- clinical row: nav-row label id `0x3A` with its English pointer patched to the code-cave `AIRBREAK_CLINICAL_LABEL` string
- payload: `patches/templates/my_options_essentials_mask_fit_patch.c`
- CRC: all three firmware CRC segments are recomputed and verified

The payload preserves the original lower My Options append, constructs AirBreak navigation rows from the screen model, and appends them
while there is spare capacity. The row order comes from the AirBreak UI model. In the default model, Block Breaker is placed
above Custom About, Custom About is placed directly above Clinical Mode, and Clinical Mode is the last item in the visible
default and `Essentials=Plus` expanded My Options lists.
Custom About does not reuse the stock
About label, route to the stock About page, or rewrite any About-page text. It routes to the AirBreak-owned
custom page and shows `This is Custom About` by default. Block Breaker routes to the AirBreak-owned page host,
sets the title to `Block Breaker`, and then draws an Atari-style full-frame game as the owned LCD surface. The active game page has no stock Back row or control rows, and the event setter gate blocks stock page, row, and selected-row writes while the game is active, so encoder rotation/clicks cannot drive the hidden stock UI behind the game. On entry, the game drains a short input window so encoder rotation used to reach/select the menu cannot become the first paddle move. Paddle input then comes from the firmware rotary provider or raw PF10/PF11 encoder phase reads, and consumed rotary state is cleared before the stock UI can reuse it. Fire comes from the PG11 encoder-button edge, the ball advances on the post-render tick without requiring another button press, and PG7 Home first restores the stored My Options origin, clears the game SRAM state, releases the rotary provider back to the stock UI, and then lets the stock Home handler complete the exit. The ball uses a finer 16x16 logical grid, while the bricks use an independent 18-bit `6x3` SRAM map at `0x2001FCC0` so one collision clears one brick. Each AirBreak row
stores its My Options origin in the row action object, the entry action copies that origin and the current selected row into
an AirBreak-owned SRAM scratch word, and the owning screen's Back action reads that scratch state before returning. Custom
About and Block Breaker no longer share a Back action; Block Breaker owns its cleanup and input release path. The Clinical Mode row dispatches the same stock
hidden clinical-entry action used by the Home+encoder shortcut through the firmware's clinical input host, so it enters the stock Clinical Menu root instead of
guessing a page index. Any crash or invalid state in the
resulting firmware should also be observable through the local emulator path.

## Layout

- `patches/tools/`: local firmware patch and payload build tools.
- `patches/templates/`: code-cave payload source and linker script.
- `scripts/run-station-pipeline.sh`: reverse/patch/emulate orchestration entrypoint.
- `scripts/run-png-regressions.sh`: static PNG baseline regression runner.
- `scripts/run-gui*.sh`: emulator launch and regression wrappers.
- `scripts/lib/airbreak_ui_model.sh`: shared AirBreak UI screen model and target navigation helpers.
- `scripts/lib/rust_emulator_env.sh`: Rust STM32 emulator bootstrap using the vendored source tree.
- `tests/png-regressions/`: per-firmware PNG regression manifests and committed baselines.
- `rust/airbreak-f405.yaml.in`: AirBreak emulator config template.
- `rust/stm32-emulator/`: vendored STM32 emulator source with AirBreak support applied in code.
