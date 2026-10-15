# assets

| File | What it is |
| --- | --- |
| `icon.svg` | The icon, and the only copy that is edited |

The mark is "A 合流" (a meeting of ways), chosen on 2026-10-15 from the three
in the screen mock: a branch joining main, drawn the way the commit graph keeps
main on its left lane. The tile and the colors are tsumugi Dark's, the same as
kura's icon. Inside the window the title bar draws the mark alone in the
theme's colors; the mock ([docs/mock/michi.html](../docs/mock/michi.html))
reads this file.

When the code starts (v0.1.0), `icon.ico` for Windows is generated from the SVG
the way kura does it (`cargo run --example make-icon`), and the window's icon is
rasterized from the SVG at startup.

## Licensing

The rest of this repository is under MIT or Apache-2.0, at your option. **The
artwork in this directory is not part of that grant.** It is the project's mark
rather than its code, and is kept here so the program can be built with it, not
so it can be reused elsewhere. A fork should replace `icon.svg` with its own.
