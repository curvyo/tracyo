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
| `icons/lucide/LICENSE` | `b495047bd93a9b06913511076f504daba17d5bbeb3e0650f3bb53a4220329c57` |
