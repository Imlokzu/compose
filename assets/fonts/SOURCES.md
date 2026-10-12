# Bundled font sources

Static TTFs are registered with Parley and GPUI from the same bytes. Downloaded
from the upstream repositories on 2026-10-09; no system fonts are required for
these six families.

- Lato (Light, Regular, Bold, Black + corresponding italics):
  https://github.com/google/fonts/tree/main/ofl/lato — OFL.txt
- Noto Serif (Regular/Bold + italics), Noto Sans (Regular/Bold + italics),
  Noto Sans Mono (Regular/Bold):
  https://github.com/notofonts/noto-fonts/tree/main/hinted/ttf — Noto-LICENSE.txt
- Anton Regular: https://github.com/google/fonts/tree/main/ofl/anton — anton-OFL.txt
- Bebas Neue Regular: https://github.com/google/fonts/tree/main/ofl/bebasneue — bebasneue-OFL.txt

Only provided weights/styles are offered by the picker. Additional scripts still
use system fallback fonts; cross-platform fallback identity is not guaranteed.

Use typographic family name ID 16 before legacy ID 1 when passing a Parley face to
GPUI. In these files, Lato Light/Black and Noto Serif Bold/Italic have legacy names
such as `Lato Light` or `Noto Serif Bold`, while their registered native families
are `Lato` and `Noto Serif`. Weight and italic remain separate attributes. Native
validation checks every bundled face against the glyph indices from these bytes.

Android document previews/export use Google's Noto Color Emoji CBDT font,
Unicode 18 release commit e20cbc2bbec1926686be9f9bee7d1d2cfa1fea0e,
from https://github.com/googlefonts/noto-emoji/tree/main/2D/fonts .
SIL OFL license: Emoji-OFL.txt. The pinned bitmap-color font works with the
shared Swash renderer; Android 15's system COLRv1 font is not supported there.
It is compiled into Android's native library only; mobile keyboard input uses
Android's own emoji rendering. System Arabic/CJK/Devanagari fallback fonts are
read from the device when present and are not redistributed.
