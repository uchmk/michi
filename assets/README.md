# assets

| File | What it is |
| --- | --- |
| `icon.svg` | The icon, and the only copy that is edited |
| `screenshots/*.png`, `demo.gif` | Window captures of `michi.exe` (v0.23.0) on the tsumugi repository, for the top README |
| `social-preview.png` | The 1280×640 image for the repository's social preview (set by hand in Settings; GitHub has no API for it) |

The mark is "A 合流" (a meeting of ways), chosen on 2026-10-15 from the three
in the screen mock: a branch joining main, drawn the way the commit graph keeps
main on its left lane. The tile and the colors are tsumugi Dark's, the same as
kura's icon. Inside the window the title bar draws the mark alone in the
theme's colors; the mock ([docs/mock/michi.html](../docs/mock/michi.html))
reads this file.

The program does not read the SVG: `crates/michi/src/icon.rs` draws the same
picture in code at any size, for the window's icon and the title bar's logo,
and `crates/michi/build.rs` bakes it into `michi.exe` at eight sizes with
ito's `ito-winres` (the way tsumugi does it). There is no `.ico` to keep in
step. A change to `icon.svg` is copied into `icon.rs` by hand.

## Licensing

The rest of this repository is under MIT or Apache-2.0, at your option. **The
artwork in this directory is not part of that grant.** It is the project's mark
rather than its code, and is kept here so the program can be built with it, not
so it can be reused elsewhere. A fork should replace `icon.svg` with its own.
