# Milton Hardware

> [YMMV][ymmv]; Get creative with the esp32c3 mounting. Soldering some female headers to a cute little
> electrocookie protoboard was satisfactory enough for me. At the end of the day we're just using a single
> logic pin, and the 3.3v/ground connections.

## Purchase List

| Name/Link | Price |
| --- | --- |
| [xiao-esp32c3][xiao] | ~$5.00 USD |
| [electrocookie mini pcb][pcb] | ~$10.00 USD (pack of 6) |
| ws2812 led strip ([example][led]) | varies, ~$9.00 USD |
| m3 screws (various lengths) | [1k piece kits available on amazon for ~$20.00][m3] |

## Printed Parts

| File | Quanitity | Notes |
| --- | --- | --- |
| `generic-rail-attachment-base.stl` | 3 | This part is re-used as a way to provide a secure base to various attachments using m3 screws. |
| `filament-guide-with-wheel-slot.stl` | 1 | When using a top-mounted spool holder like [this one][fila], the filament being fed through the material sensor unit may erode away its plastic. This part helps keep the filament being fed at an angle to minimize this problem |
| `led-strip-rail-attachment-base.stl` | 1 | Attaches to a `generic-rail-attachment-base` and provides m3 screw holes for the `led-strip-main-rail`. |
| `led-strip-main-rail.stl` | 1 | The main led strip holder. During assembly, it may be helpful to use velcro instead of any adhesive that came with the led strip. |
| `electrocookie-protoboard-holder.stl` | 1 | The xiao-esp32c3 is connected to our strip of neopixel lights via a simple protoboard |


Other Printed Parts:

1. [Anycubic Vyper Filament Top Rail Mount][fila]
2. [45 Degree Adapter](https://www.thingiverse.com/thing:4943235)

| Xiao Protoboard |
| --- |
| ![IMG_0331][pcb-photo] |

[← Readme](../README.md)

[fila]: https://www.thingiverse.com/thing:5407700
[xiao]: https://www.seeedstudio.com/Seeed-XIAO-ESP32C3-p-5431.html
[pcb]: https://www.amazon.com/dp/B081MSKJJX
[led]: https://www.amazon.com/dp/B097BX7LRT
[m3]: https://www.amazon.com/dp/B098B2XN7X
[ymmv]: https://dictionary.cambridge.org/us/dictionary/english/ymmv
[pcb-photo]: https://user-images.githubusercontent.com/1545348/208219958-b66acc25-5039-432c-861e-548a559d13dc.jpg
