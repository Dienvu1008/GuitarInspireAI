# Native Project Format

Purpose: Specify the native project package: structure, manifest, versioning, serialization, migration, assets, revisions, integrity, import and export boundaries, compatibility, atomic save, recovery, ZIP safety, unknown-field preservation and the round-trip test suite.
Status: Accepted (ZIP + JSON with versioned DTOs). File extension: open (Q-08). Windows atomic replace details: [To verify] in S5.
Related documents: [data-architecture.md](data-architecture.md), [domain-invariants.md](domain-invariants.md), [security-and-privacy.md](security-and-privacy.md), [ADR-008](decisions/ADR-008-native-project-format.md), [technical-spikes.md](../research/technical-spikes.md).

## 1. Container alternatives

| Option | Pros | Cons | Verdict |
|---|---|---|---|
| Single-file ZIP package | One file to share, email, sync; established (DOCX, GP7 use zipped content); random entry access | Not diffable in git; rewriting the whole file on save; large audio makes saves slow | Selected for MVP |
| Directory project | Diffable, large audio files untouched on save, easy recovery | Users move or partially copy folders; harder to share; cloud sync partial states | Reconsider when audio projects become common |
| SQLite as document | Transactional, incremental writes | Opaque, migration via SQL, binary blobs, harder to inspect | Rejected for canonical score |

Reconsideration trigger: projects with recorded audio over about 100 MB [Assumption] or users asking for version control friendliness. A hybrid (ZIP for score, external referenced audio folder) is the likely next step.

## 2. Package structure

```text
song.<ext>                 (ZIP, extension TBD)
  manifest.json            required, first entry
  score.json               required
  patterns/                Phase 6a, one JSON per pattern
  assets/                  audio and images, STORED (no compression)
  analysis/                reserved: accepted analysis results with provenance
  history/                 reserved: named revisions (not undo)
```

Unknown top-level prefixes are rejected unless listed in `optional_features`, so a future version cannot smuggle unvalidated content.

## 3. Manifest (example)

```json
{
  "format": "guitarinspire.project",
  "format_version": { "major": 1, "minor": 0 },
  "required_features": [],
  "optional_features": [],
  "project_id": "01J9Z3K8Q4V6T2H0M5N7P8R9S1",
  "created_by": { "app": "GuitarInspireAI", "version": "0.1.0" },
  "saved_by": { "app": "GuitarInspireAI", "version": "0.1.0" },
  "saved_revision": 42,
  "entries": [
    { "path": "score.json", "sha256": "<hex>", "size": 18234, "media_type": "application/json" }
  ]
}
```

- `format_version.major`: incompatible changes; requires migration.
- `format_version.minor`: additive optional fields.
- `required_features`: capabilities a reader must understand (for example `polymeter`). Unknown required feature: refuse to open.
- `optional_features`: capabilities that can be ignored but must be preserved.
- Ids are ULIDs or UUIDv7 [Proposed]; stable across saves.

## 4. Serialization

- JSON via `serde` into persistence DTOs in `mc-project` (`dto::v1::*`), then `TryFrom<Dto> for Score` through `ScoreBuilder` (P-05).
- Deterministic output: stable field order (struct order), maps as `BTreeMap`, no floats in music data (rationals as `[num, den]`), UTF-8, newline-terminated. Deterministic output enables byte-stable golden tests and minimal sync diffs.
- Rationale for JSON over binary (CBOR, bincode): inspectable, debuggable, tolerant to added fields, sufficient size when deflated (S5 measures).

## 5. Versioning, migration and compatibility policy

| Case | Behaviour |
|---|---|
| Same major, same or older minor | Load directly; missing optional fields get defaults |
| Same major, newer minor | Load; unknown fields preserved (section 9); save keeps the newer minor number if any preserved content exists [Proposed] |
| Older major | Migrate forward through a chain `v1 -> v2 -> ...` of pure functions on JSON values or DTOs; original file untouched until the user saves; first save after migration writes a `.bak` of the original |
| Newer major | Refuse with a clear message naming the required app version |
| Unknown required feature | Refuse with message |
| Unknown optional feature | Load, preserve, warn |
| Downgrade (save for older app) | Not supported in MVP |

Backward compatibility commitment: every released format version stays loadable by all later app versions (enforced by FF-11 golden corpus). Forward compatibility: best effort within the same major.

## 6. Asset references

- Assets are referenced by id; manifest maps id to entry path and hash.
- External references (Proposed, later): `{ "kind": "external", "relative_path": "...", "sha256": "..." }`; missing external files are a load warning, not an error.
- Audio is stored uncompressed in the ZIP (STORED) to avoid deflate cost on already-compressed or large PCM data.

## 7. Revision history

- Undo history is not persisted.
- `saved_revision` is recorded for diagnostics and sync.
- Named revisions (`history/`) are Deferred; when added, each is a full score snapshot JSON with a timestamp and label.

## 8. Integrity validation on load

1. Container checks (section 11).
2. `manifest.json` parse and version policy.
3. Every manifest entry exists; every package entry is either in the manifest or rejected.
4. SHA-256 per entry matches (P-02). Mismatch: typed error; user may attempt "open anyway" in read-only mode [Proposed].
5. DTO parse, then domain construction via `ScoreBuilder` (all hard invariants).
6. Diagnostics computed and shown (do not block).

## 9. Unknown-field preservation

Serde drops unknown fields by default. Preservation is not automatic at nested levels.

Policy:

- Every persistence DTO struct, at every nesting level, has `#[serde(flatten)] extra: BTreeMap<String, serde_json::Value>`.
- Caveats: `flatten` is incompatible with `deny_unknown_fields`, costs performance, and interacts poorly with internally tagged enums. DTO enums therefore use adjacent tagging (`{"kind": ..., "data": ...}`) [Proposed, S5 checks performance].
- Unknown enum variants: designated extensible enums (for example note effects) use a custom deserializer that maps unknown variants to `Unknown { kind: String, raw: Value }` and preserve them. Other enums treat a new variant as a major change.
- Preserved unknown data is carried alongside domain objects by id (for example a side map in the loaded project keyed by note id) and re-emitted on save. If the owning object is deleted, its unknown data is dropped.
- If unknown content cannot be preserved (for example unknown data attached to a structure the user restructured), the app opens with a warning and forces Save As instead of overwrite.
- MVP scope (revised in review): while only format 1.0 exists, unknown content can only come from a newer minor version. MVP implements the `extra` maps on all DTOs and the force-Save-As rule whenever unknown content is present. Re-emitting preserved data next to edited domain objects (the side map) is implemented together with the first 1.1 format change, when real fixtures exist.
- FF-11 includes a test that injects an unknown field into every DTO type (generated from the schema snapshot) and asserts it survives load and save.

## 10. Atomic save on Windows

Sequence:

1. Take a structural-sharing snapshot of the score (cheap); serialize on a background thread.
2. Write to a temp file in the same directory as the target (`.song.<ext>.tmp-<random>`), so the rename stays on one volume.
3. Flush and `sync_all` (FlushFileBuffers on Windows).
4. If a previous file exists and backups are enabled, keep it as `.bak` (single generation, configurable).
5. Replace the target. Candidates: `std::fs::rename`, which on Windows uses `MoveFileExW` with replace-existing semantics [To verify in Rust std docs]; `ReplaceFileW` to preserve attributes and ACLs [To verify]; or `tempfile::NamedTempFile::persist` [To verify]. S5 selects one.
6. On success, record saved revision and emit `DocumentSaved`.

Failure modes and handling:

| Failure | Handling |
|---|---|
| Target locked (antivirus scan, OneDrive sync, another app) - sharing violation | Bounded retry with backoff (for example 5 attempts over about 2 s [Assumption]); then typed error; original untouched; temp removed |
| Disk full | Typed error; temp removed; original untouched |
| Network share or FAT volume (rename not atomic) | Detect if possible; warn; still use temp and replace |
| Crash during write | Temp file left; cleaned on next start; original intact |
| Crash between backup and replace | Recovery finds `.bak` and temp; offers restore |

## 11. ZIP read safety

- MVP never extracts to the file system: entries are read into memory.
- Reject: absolute paths, `..` segments, drive letters, backslashes, symlink entries, duplicate names (case-insensitive comparison on all platforms), entries not in the allowed structure, encrypted entries, compression methods other than STORED and DEFLATE.
- Limits (configurable defaults, justified by the large reference score of about 15 to 30 MB in memory and JSON overhead): max entries 10 000, max uncompressed score.json 256 MB, max total uncompressed 2 GB (audio), max compression ratio 200:1 for deflated entries [Assumption, tune in S5].
- Limits are enforced on actual bytes read from a bounded reader, not on header-declared sizes.
- ZIP64: accepted for reading assets over 4 GB? Decision: reject over the total limit anyway; ZIP64 headers accepted if within limits [Proposed].
- Use `zip` crate with `default-features = false` and deflate only ([technology-evaluation.md](../research/technology-evaluation.md)).

## 12. Backup and crash recovery

- Autosave: every 2 minutes when dirty (configurable), and on app backgrounding or window close prompts, to `recovery/<project-id>/autosave-<n>.<ext>` using the same atomic write. Retain 3 rotating files.
- Session marker: a lock file in app data written at start and removed on clean exit. If present at start, an unclean shutdown happened.
- On start after unclean shutdown: list recovery files newer than their source project and offer Restore, Open original, or Discard.
- Untitled documents autosave to recovery as well.
- Recovery files are deleted after a successful normal save of the source.

## 13. Import and export boundaries

The native loader is the only path that produces a Score from native files. Importers live in adapter modules, use `ScoreBuilder`, and produce reports. Exporters read snapshots and never mutate. No importer writes the native format directly.

## 14. Round-trip test suite (FF-11, FF-32)

1. Every fixture of every released version loads and validates.
2. Load, save, load gives semantic equality.
3. Current version output is byte-stable against golden files.
4. Unknown fields injected at every DTO level survive load and save.
5. Migration chain output equals expected fixtures for each older version.
6. Newer major version and unknown required features are refused with typed errors.
7. Corrupted hash, truncated ZIP, traversal entries, duplicate entries and decompression bombs produce typed errors and no panic.
8. DTO JSON schema snapshot (schemars) diff fails CI unless the format version changed or the change is marked additive in the changelog.
9. Fuzzing of the loader with cargo-fuzz on Linux CI (Windows libFuzzer support is limited [To verify]).
