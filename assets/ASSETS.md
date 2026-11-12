# Assets

Everything under `assets/` is embedded into the executable. Each file is named below with its
source, licence and SHA-256; `crates/tracyo-app/tests/assets.rs` checks the list against the
files and the model against the hashes `vectrace::ocr` expects.

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
| `icons/lucide/ban.svg` | `997ff9d478e9476f03779718c246bf194415905959a6e01060f8149f42e0c027` |
| `icons/lucide/pipette.svg` | `63070d715737dbfe3d588700c6af70fbd2d570ec5aa1637196d7c042b6d6f10a` |
| `icons/lucide/contrast.svg` | `35d940bd31e082481cad07838b8271185529d50bbd424fe86e3214cb90736d3a` |
| `icons/lucide/image-minus.svg` | `3f31b92b73b8ff7963263f280684dcea7617e4fae5c39edd320638678242b67b` |
| `icons/lucide/chevrons-down.svg` | `2bdcfe344b1a24a5937388a4619f533caaa5faae30828e442ebf0175b575c0ce` |
| `icons/lucide/scan-text.svg` | `07bc3db82d4f9d0ca5e79fdfff7dde32cba2f03db3a54482bef69176f563df72` |
| `icons/lucide/shapes.svg` | `c194534da0733adc59e3a3cc52c4055f65cdbedd50bff181643104b24987acd7` |
| `icons/lucide/chevrons-up.svg` | `6df1d2dc6e8141e81c0e45ff9a57c17dfbc3c49df2b0ceabe9a8ec3f60f1b3c0` |
| `icons/lucide/square.svg` | `b039603faf398f9a953ac245ff7a03eae11009b81f7130bf38a4bda819b03c10` |
| `icons/lucide/square-minus.svg` | `0956033f77c18eb42b6fcc4901d3405aeab13872d4b8bef67b72a7b86fca002d` |
| `icons/lucide/plus.svg` | `24fad69418db84378f6d368e7609d85f89364381a5ecbc7f5c10687d9204f18c` |
| `icons/lucide/trash-2.svg` | `58049d9ef9a117aba81f01c9c11e1fbfb4a31df43163e82f07cba65e3c507e5a` |
| `icons/lucide/LICENSE` | `b495047bd93a9b06913511076f504daba17d5bbeb3e0650f3bb53a4220329c57` |

## OCR model: PaddleOCR Latin (mobile), Apache-2.0

The ONNX files of `vectrace` v0.1.0 (`assets/models/paddleocr-latin/`), copied unmodified; they are
embedded for Recognise text (ADR 0006), nothing is downloaded. `model.toml` is vectrace's licence
record (source URL and SHA-256 per file), `LICENSE` the Apache-2.0 text. Copy the files again from
the new tag when a vectrace bump changes `vectrace::ocr::DETECTOR_SHA256`, `RECOGNISER_SHA256` or
`ALPHABET_SHA256`; the test fails until then.

| File | SHA-256 |
|---|---|
| `models/paddleocr-latin/det.onnx` | `ee40e80071ba3a320d4efda75f3e22047a7d049e9bf7bcaaf9daea23fc21b935` |
| `models/paddleocr-latin/rec.onnx` | `614ffc2d6d3902d360fad7f1b0dd455ee45e877069d14c4e51a99dc4ef144409` |
| `models/paddleocr-latin/dict.txt` | `3c0a8a79b612653c25f765271714f71281e4e955962c153e272b7b8c1d2b13ff` |
| `models/paddleocr-latin/model.toml` | `a33fe51c751c9dbf88e8a492fe8cfb6385b0fa1a3d7d76fbcc0a8902c0ececdf` |
| `models/paddleocr-latin/LICENSE` | `cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30` |

## Test pictures (not embedded)

`crates/tracyo-core/tests/fixtures/pictures/precision.png` is `assets/trace-references/precision.png`
of vectrace v0.1.0 (CC0 1.0, a customer logo with the text "PRÄZISION", "MIT SYSTEM" and a line of
three words; SHA-256 `c2584eb4f30c7adda1d6bf0b5da8a8329f6c798815b2816f074d515e242186e2`). It is the
picture of the OCR tests. The other files there are small pictures made for these tests.
