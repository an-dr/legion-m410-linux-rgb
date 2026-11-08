# M410 protocol

How the Legion M410 is controlled, as worked out by probing the mouse. Lenovo
publishes nothing, so treat this as observed behavior, not a specification. The
implementation is [crates/m410](../crates/m410).

## Transport

The mouse (`17ef:6197`, USB cable) and its wireless receiver (`17ef:6198`) both
expose a vendor HID collection on USB interface 1 (usage page `0xFF1C`). It takes
64-byte output reports with ID 4 and answers with the same.

```text
04 00 00 <cmd> <len> <offset lo> <offset hi> 00 <data...>
```

The reply repeats the header, puts a status byte at index 7 (`00` = accepted)
and, for reads, the data from index 8.

| Command | Meaning |
| --- | --- |
| `05` | read `len` bytes of the configuration page at `offset` |
| `06` | write to the configuration page |
| `0d` | factory reset: the LEDs blink and every setting returns to its default |

Over the receiver a transfer carries at most `0x18` bytes of data; the cable
allows `0x38`. The page moves in `0x18`-byte chunks on both.

## The configuration page

Settings live in one `0x80`-byte flash page. A write that starts at offset 0
erases the page first. A write anywhere else can only clear bits, because the
firmware does not erase for it, so a changed byte may silently not stick. Every
change therefore rewrites the whole page from offset 0, then reads it back to
check. The mouse applies the new lighting immediately and keeps it across power
cycles.

| Offset | Meaning |
| --- | --- |
| `01` | effect: `01` static, `02` breathing, `03` color cycle, `04` rainbow wave (`00`, the factory value, behaves as breathing) |
| `02` | brightness: `00` off, `01`–`04` |
| `03` | animation period: `00` fastest to `04` slowest |
| `05` | `ff`: a new random color each cycle; `00`: use the color below |
| `06`–`08` | red, green, blue |
| `0e`–`4c` | seven 9-byte DPI stages (enabled, X and Y DPI, indicator color), untouched by this app |

Cycle and wave pick their own colors, so `05`–`08` do nothing for them. Both
LED zones always show the same color, except in the wave effect.

## Sleep

A wireless mouse sleeps about 30 s after it was last moved. Asleep, it ignores
every request through the receiver; requests neither wake it nor keep it awake.
The timeout is not stored in the configuration page: changing bytes `09`–`0d`,
`4d` and `78`–`7f` left it unchanged.
