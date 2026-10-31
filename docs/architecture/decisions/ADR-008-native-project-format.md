# ADR-008 Native project format: ZIP + JSON with versioned DTOs

Status: Accepted (2026-10-31, after S5), with the amendments below
Date: 2026-10-31
Related documents: [project-format.md](../project-format.md), [data-architecture.md](../data-architecture.md), [technical-spikes.md](../../research/technical-spikes.md)

## Context
Interchange formats cannot hold the full domain. Users need a single shareable file. The format must evolve safely.

## Decision
Single-file ZIP package with `manifest.json`, `score.json` and reserved folders. JSON via serde into versioned DTOs in `mc-project`, converted to domain types through validating builders. Major and minor versioning, required and optional features, unknown content handled in three levels (U1 detection via `flatten` on every DTO and U2 Save As protection in MVP; U3 full preservation with the first 1.1 format, see project-format section 9), atomic save with backup, autosave recovery, strict ZIP reading limits.

Amendments from S5 evidence:

1. Replace step: write and `sync_all` a temp file in the target directory, then `std::fs::rename` with bounded retry (5 attempts, backoff from 100 ms, about 1.5 s total). It succeeded with readers holding full-sharing handles and recovered after a transient lock. `tempfile::persist` is rejected because it failed while a reader held the file open. `ReplaceFileW` is an optional Windows refinement that also preserves creation time; not needed for MVP.
2. `zip` is used with `default-features = false` and only `deflate-flate2-zlib-rs`. `ZipFile::name()` in zip 9 returns a `Result`; name errors map to the same typed load error as our own name rules.
3. Default decompression ratio cap is 100:1, measured against a 17.5:1 ratio for the large reference score. The cap bounds memory before rejection (a 1 GiB bomb was rejected after 104 MB).

Amendments from Phase 1 implementation evidence:

4. Retries are classified by portable `std::io::ErrorKind` (`PermissionDenied`, `ResourceBusy`), not by raw OS error codes. The S5 spike matched Windows codes 5 and 32, but the same numbers mean different errors on Linux (EIO, EPIPE), and FF-07 keeps platform `cfg` out of `mc-app`. On Windows the S5 lock case surfaces as `PermissionDenied`; tests on the reference machine confirm retry until release and a typed error after the last attempt. Cost: a genuine permission error is reported after about 1.5 s instead of immediately.
5. Measured (Phase 1, reference machine): open typical 24 ms and large 387 ms; save typical 17 ms and large 342 ms, including SHA-256, U1 detection and full validation.

## Alternatives
Directory project; SQLite document; binary serialization (CBOR, bincode); MusicXML or GP as canonical.

## Trade-offs
Shareable and inspectable; whole-file rewrite on save and not git-diffable. Open and save are fast in practice (large reference score: open under 250 ms, save about 120 ms). Unknown content detection costs about 2x JSON parse time and adds DTO complexity; until U3 exists, files from newer minor versions can only be saved with Save As.

## Consequences
DTOs are a deliberate second representation. A fixture corpus per released version is maintained forever.

## Related fitness functions
FF-11, FF-15, FF-32.

## Reconsideration conditions
Large audio projects (save times over budget); users need version control friendliness; collaboration; real OneDrive or antivirus behaviour (not tested in M0) defeats the retry policy.
