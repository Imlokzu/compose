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
