# Bundled font provenance (ADR-037)

| Field | Value |
|---|---|
| Font | Bravura, SMuFL music font |
| Release | `bravura-1.482` (2026-08-24), https://github.com/steinbergmedia/bravura/releases/tag/bravura-1.482 |
| License | SIL Open Font License 1.1, see [OFL.txt](OFL.txt) (copied unmodified from the release) |
| Copyright | © 2015, Steinberg Media Technologies GmbH. Reserved Font Name "Bravura" |
| Metadata | `Bravura.json`: `fontName` "Bravura", `fontVersion` 1.482 |

Files, unmodified, upstream names. FF-35 checks every row of this table against the bundled bytes:

| File | Size (bytes) | SHA-256 |
|---|---|---|
| `Bravura.otf` | 889228 | `CDF0F893EE1FDB64B7F6713D71EE0DCFC349C0AC01429A8E451B01A9E79F5F3B` |
| `Bravura.json` | 1256995 | `489FB4D734FC0B72A3CC601B50EBDF0BF8C57802AB9893E52CE030B86BF2CE11` |
| `OFL.txt` | 4420 | `C24929BE7028026A65EE8894DA1E3C36D2A4CCCE0548D9BA8BA64509F46319EE` |

Use:
- standard notation in the app;
- embedding a subset in exported PDFs (OFL-FAQ 1.12).

The bundled file is never subset or altered, because subsetting counts as modification (OFL-FAQ 2.6). The font is bundled with the application, never sold by itself, and listed in the release third-party notices with its license.

`FONTLOG.txt` and `OFL-FAQ.txt` of the release were audited (ADR-037) and are not bundled.
