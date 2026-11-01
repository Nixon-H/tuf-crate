# Keyboard RGB — protocol, effects, speed and brightness

**Status: working on FX706HF (BIOS 312, kernel 7.2.11).** Verified live from Linux through
the upstream `asus-wmi` interface — no EC poking, no AsIO, no vendor drivers.

## Interface

```
/sys/class/leds/asus::kbd_backlight/kbd_rgb_mode    (write-only)
  <cmd> <mode> <r> <g> <b> <speed-index>

  cmd         1 = 0xb4 ("set"), 0 = 0xb3
  mode        0..11 (see table; 9 and >=12 are mapped to 10 by the kernel)
  r/g/b       0..255 each — arbitrary mixing
  speed-index 0 = 33 %, 1 = 67 %, 2 = 100 %  (kernel maps to 0xe1/0xeb/0xf5)
```

The kernel `asus-wmi` driver issues:

```
WMNB DEVS(0x00100056, 0xb4 | mode<<8 | R<<16 | G<<24, B | speed<<8)
```

and requires the keyboard power-state command first:

```
/sys/class/leds/asus::kbd_backlight/kbd_rgb_state
  <cmd> <boot> <awake> <sleep> <shutdown>     e.g. "1 1 1 0 0"
```

Without the state command the firmware acknowledges effect modes but renders them static.
tuf-crate sends `1 1 1 0 0` before every RGB apply (idempotent, same value G-Helper uses).

Brightness is independent and lives on the LED class:

```
/sys/class/leds/asus::kbd_backlight/brightness   0..3   (0 %, 33 %, 66 %, 100 %)
```

## Effects

| Mode | Name | Notes |
|---|---|---|
| 0 | Static | solid mixed colour |
| 1 | Breathing | pulse; speed applies |
| 2 | Color Cycle | hue sweep; speed applies |
| 3 | Rainbow | animated rainbow; **visually confirmed** |
| 4 | Star | twinkle |
| 5 | Rain | falling drops |
| 6 | Highlight | travelling band |
| 7 | Laser | sweeping beam |
| 8 | Ripple | expanding ripples |
| 10 | Strobe | fast flashing |
| 11 | Comet | comet + tail |

Mode 12 ("Flash") is not exposed: the kernel sanitises >= 12 to Strobe. Music / Smart /
Adaptive / AI effects are Armoury-Crate-side rendering features with no firmware hook on
this model.

## Software effect: Music (audio-reactive)

Armoury Crate's Music effect is Windows-side rendering; this project implements a
Linux-native equivalent in the Lighting tab:

- `pw-record --target auto -P stream.capture.sink=true` captures the **default output**
  (sink monitor) at 8 kHz mono s16.
- A VU-style RMS level (fast attack / slow release) and a simple beat detector are
  computed in a background thread; beats trigger a full-brightness flash.
- The GUI derives a colour from the picker hue + level (brightness 8–100 %, small hue
  shift) and streams **Static (mode 0)** frames through the normal `set_kbd_rgb` path, at
  most ~12 frames/s and only when the colour changes meaningfully.
- The effect stops when the GUI closes, when "Stop Music" is pressed, or when any hardware
  effect is selected.

Every frame goes through the same validated, capability-gated, audited funnel. Because the
frames are real writes, each one is recorded in the audit log (rate-limited by the change
threshold); the effect is fully reversible (stop it or pick a static colour).

Verified end-to-end headlessly with a 440 Hz tone: the level meter climbed and the audit
log recorded the frame sequence (e.g. `1 0 33 1 0 1` → `1 0 101 25 0 1` → `1 0 159 69 0 1`).

![Music effect running in the Lighting tab](images/lighting-music.png)

## Verification

| Check | Evidence |
|---|---|
| DEVID present | `DSTS(0x00100056)` = `0x00010000` |
| Static colour | red, green, blue, amber, violet (128,0,255) visibly confirmed |
| Colour mixing | arbitrary 0–255 RGB confirmed (violet) |
| Breathing | 33 % = slow pulse; 100 % = visibly faster pulse |
| Color Cycle | hue sweep confirmed |
| Rainbow | cycling confirmed |
| Star | twinkling confirmed |
| Rain | falling drops/streaks confirmed |
| Highlight | travelling band confirmed |
| Laser | sweeping beam confirmed (multicolour rendering) |
| Ripple | expanding ripples confirmed |
| Strobe | rapid flashing confirmed |
| Comet | moving comet with tail confirmed |
| Speed mapping | 33/67/100 visually distinct (33 slow vs 100 fast); kernel indices 0/1/2 |
| Brightness | LED levels 0–3 verified earlier |

All 11 firmware effect modes were applied to the physical keyboard and observed by the
owner. The firmware renders several effects in multi-colour regardless of the base colour
(e.g. Laser); the picker colour still controls Static/Breathing and the tint of others.

## Safety

- Only the two documented kernel attributes above are written.
- Values are validated (five/six components, each 0–255); the kernel validates mode/speed.
- No EC register access, no SMI invocation, no AsIO/ATKACPI, no firmware changes.
- Reversible: send any colour/effect again; state is volatile and resets on reboot.

## Implementation map

- `crates/tuf-core` — `VerifiedAttr::KbdRgb` (closed-enum safety boundary)
- `crates/tuf-hal` — `set_kbd_rgb()`: prefers `kbd_rgb_mode`, writes `kbd_rgb_state`,
  converts the UI speed to the kernel index, audits every write; `tuf_rgb.ko` fallback
- `crates/tuf-daemon` — `set_kbd_rgb` op over the local socket
- `crates/tuf-gui` — Lighting tab: 11 effects, speed chips, saturation/value square, hue
  strip, RGB sliders, HEX, presets, animated original-art keyboard preview, local profiles,
  brightness levels

![Lighting tab — Rainbow effect running in the animated preview](images/lighting-rainbow.png)
