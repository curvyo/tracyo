# Assets

Everything under `assets/` is embedded into the executable. Each file is named below with its
source, licence and SHA-256; `crates/tracyo-app/tests/assets.rs` checks the list against the
files. The OCR model arrives with the story that reads text (slice 4).

## Fonts: Liberation Sans 2.x, SIL Open Font License 1.1

Copied from the `ttf-liberation` package of the build machine; the licence text is `fonts/OFL.txt`.

| File | SHA-256 |
|---|---|
| `fonts/LiberationSans-Regular.ttf` | `baccc64becc3eb7d104b7c84d99f5314a0a1f896e2b3ea6c2f22fc08d2003bee` |
| `fonts/LiberationSans-Bold.ttf` | `769673c4355020b1e28a14c366a152da410ab6b16239fe883ebc35b73624835b` |
| `fonts/OFL.txt` | `93fed46019c38bbe566b479d22148e2e8a1e85ada614accb0211c37b2c61c19b` |

## Icons: lucide 1.50.0, ISC

The SVGs are written from the icon data of `lucide-react@1.50.0` (the version Curvyo uses); the
licence text is `icons/lucide/LICENSE`.

| File | SHA-256 |
|---|---|
| `icons/lucide/import.svg` | `7876ab38a006024cfd58a5c9f99ddc831c47b6645a65ac3e591e2fe47cf4cb91` |
| `icons/lucide/info.svg` | `3d0447616b12fa824d24287e246368f57c157ec877ab2eb44bfe3d229a817be8` |
| `icons/lucide/x.svg` | `3bfde5660f68acb8e7f4a530a9e2e67d47022d4240d1faa9aa2fc86251e046ca` |
| `icons/lucide/circle-alert.svg` | `e3ad2276dbb035e60bdcb42052fa6fa04e236a6966c24e31702fe93c6610cdf8` |
| `icons/lucide/pipette.svg` | `63070d715737dbfe3d588700c6af70fbd2d570ec5aa1637196d7c042b6d6f10a` |
| `icons/lucide/contrast.svg` | `35d940bd31e082481cad07838b8271185529d50bbd424fe86e3214cb90736d3a` |
| `icons/lucide/image-minus.svg` | `3f31b92b73b8ff7963263f280684dcea7617e4fae5c39edd320638678242b67b` |
| `icons/lucide/chevrons-down.svg` | `2bdcfe344b1a24a5937388a4619f533caaa5faae30828e442ebf0175b575c0ce` |
| `icons/lucide/chevrons-up.svg` | `6df1d2dc6e8141e81c0e45ff9a57c17dfbc3c49df2b0ceabe9a8ec3f60f1b3c0` |
| `icons/lucide/LICENSE` | `b495047bd93a9b06913511076f504daba17d5bbeb3e0650f3bb53a4220329c57` |
