# Native Project Format

Purpose: Specify the native project package: structure, manifest, versioning, serialization, migration, assets, revisions, integrity, import and export boundaries, compatibility, atomic save, recovery, ZIP safety, unknown content handling (detection, Save As protection, full preservation) and the round-trip test suite.
Status: Accepted (ZIP + JSON with versioned DTOs), validated by S5. File extension: `.giproj` (Q-08, decided 2026-10-31; see section 2a).
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
song.giproj                (ZIP)
  manifest.json            required, first entry
  score.json               required
  patterns/library.json    format 1.2: the pattern library (section 2b)
  assets/                  audio and images, STORED (no compression)
  analysis/                reserved: accepted analysis results with provenance
  history/                 reserved: named revisions (not undo)
```

Unknown top-level prefixes are rejected unless listed in `optional_features`, so a future version cannot smuggle unvalidated content.

## 2a. File extension

The product-level extension is `.giproj` (GuitarInspire Project). It is a naming convention for users, file dialogs and OS file association, not part of the format:

- A file is identified and checked only by its content: the `format` identifier `guitarinspire.project` and `format_version` in `manifest.json`, then the schema of each entry. A renamed file opens normally; a non-project file with the extension is rejected by its content.
- The extension is not part of the Music Core: `mc-project` reads bytes, `mc-app` accepts any path. It is defined in adapters only (`mc-cli` now, the Flutter app in Phase 3). Adapters may append it to a new file name; they never refuse a file because of its name.
- A registered media type is not needed before public distribution and stays open.

## 2b. Format versions and the pattern library

| Version | Change | Decision |
|---|---|---|
| 1.0 | First release (score, manifest) | ADR-008 |
| 1.1 | Optional note techniques (accent, palm mute, string-percussive) and beat stroke direction | ADR-035 |
| 1.2 | The pattern library, entry `patterns/library.json` | ADR-034 sections 8 and 22 |

The library entry (format 1.2):

- One JSON entry for the whole library: the id counter `next_id`, the gestures and the patterns, in ascending id order. One entry, not one per pattern as first sketched here, because patterns refer to gestures by id and the library is validated as a whole.
- It follows section 4: struct order, no floats (rationals as `[num, den]`, values as in `score.json`), defaults omitted, `extra` at every level for U1. Enums with data are structs with a `kind` field and the fields of that kind; any other field is a P-05 error with its path.
- On load, the shape is checked first (P-05, every problem with a path under `library`), then the library is rebuilt through `PatternLibrary::from_parts`, which checks every library rule (PT-01, PT-03 to PT-08). Errors: `LoadError::Invalid` for the shape, `LoadError::InvalidLibrary` for the rules.
- The entry is written only when the document has a library to store. `mc-app` writes none while the library is still the built-in sample library, so a document that never used patterns opens in a 1.1 build without unknown content.
- A 1.1 reader sees the entry as unknown content (`package:patterns/library.json`, U1) and refuses to save over the file (U2).
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
- `optional_features`: capabilities a reader may ignore. Unknown ones are detected (U1) and trigger Save As protection (U2) until full preservation (U3) exists (section 9).
- Ids are ULIDs or UUIDv7 [Proposed]; stable across saves.

## 4. Serialization

- JSON via `serde` into persistence DTOs in `mc-project` (`dto::v1::*`), then `TryFrom<Dto> for Score` through `ScoreBuilder` (P-05).
- Deterministic output: stable field order (struct order), maps as `BTreeMap`, no floats in music data (rationals as `[num, den]`), UTF-8, newline-terminated. Deterministic output enables byte-stable golden tests and minimal sync diffs.
- Rationale for JSON over binary (CBOR, bincode): inspectable, debuggable, tolerant to added fields, sufficient size when deflated (S5 measures).

## 5. Versioning, migration and compatibility policy

| Case | Behaviour |
|---|---|
| Same major, same or older minor | Load directly; missing optional fields get defaults |
| Same major, newer minor | Load; unknown content detected (U1) and the original protected by Save As (U2); full preservation (U3) once implemented (section 9) |
| Older major | Migrate forward through a chain `v1 -> v2 -> ...` of pure functions on JSON values or DTOs; original file untouched until the user saves; first save after migration writes a `.bak` of the original |
| Newer major | Refuse with a clear message naming the required app version |
| Unknown required feature | Refuse with message |
| Unknown optional feature | Load, warn, apply U2 (U3 once implemented) |
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

## 9. Unknown content: detection, Save As protection, full preservation

Serde drops unknown fields by default, and nothing is preserved automatically at nested levels. The policy has three levels. This section is the single source for them; other documents refer here.

| Level | What it does | When | Tested by |
|---|---|---|---|
| U1 Detection | Every persistence DTO struct, at every nesting level, captures unknown keys with `#[serde(flatten)] extra: BTreeMap<String, serde_json::Value>`. Designated extensible enums (for example note effects) map unknown variants to `Unknown { kind, raw }`. Unknown `optional_features` and unknown entries under listed optional prefixes are also detected. After parsing, the loader walks the DTO tree and returns `LoadWarning::UnknownContent { json_paths }` | MVP (Phase 1) | Round-trip item 4a |
| U2 Save As protection | If U1 found anything, the session marks the document `preservation_incomplete`. Save to the original path is disabled. Save As to a new path is allowed, after a warning that the unknown content will not be written. The original file is never overwritten by a build that cannot write its content back | MVP (Phase 1) | Round-trip item 4b |
| U3 Full preservation | Unknown data is carried next to domain objects by id (side map keyed by object id) and re-emitted on save. If its owning object is deleted, the data is dropped. If it cannot be attached (the owner was restructured), U2 applies | Built together with the first 1.1 format change, when real fixtures exist. Not in MVP | Round-trip item 4c (from format 1.1) |

Notes:

- Other enums treat a new variant as a major change (refused by version policy, not by U1).
- `flatten` caveats: incompatible with `deny_unknown_fields`, has a parsing cost, and interacts poorly with internally tagged enums. DTO enums therefore use adjacent tagging (`{"kind": ..., "data": ...}`). S5 measures the cost.
- While only format 1.0 exists, unknown content can only come from hand-made test fixtures or a future 1.x writer. Tests therefore use generated fixtures that inject an unknown field at every DTO level.

## 10. Atomic save on Windows

Sequence:

1. Take a structural-sharing snapshot of the score (cheap); serialize on a background thread.
2. Write to a temp file in the same directory as the target (`song.giproj.tmp-<pid>-<n>`), so the rename stays on one volume.
3. Flush and `sync_all` (FlushFileBuffers on Windows).
4. If a previous file exists and backups are enabled, keep it as `.bak` (single generation, configurable).
5. Replace the target with `std::fs::rename` and bounded retry (S5 decision). On the reference machine it replaced the file while another reader held it open with full sharing, failed with access denied (os error 5) only against a handle without FILE_SHARE_DELETE, and succeeded on retry once that handle closed. `tempfile::NamedTempFile::persist` is rejected (it failed with a full-sharing reader open). `ReplaceFileW` behaved like rename and also preserved creation time; it is an optional Windows refinement.
6. On success, record saved revision and emit `DocumentSaved`.

Failure modes and handling:

| Failure | Handling |
|---|---|
| Target locked (antivirus scan, OneDrive sync, another app) - access denied or sharing violation | Bounded retry with backoff (5 attempts, 100 ms doubling, about 1.5 s; validated in S5 with a simulated lock); then typed error; original untouched; temp removed. Real OneDrive and antivirus holds not yet tested |
| Disk full | Typed error; temp removed; original untouched |
| Network share or FAT volume (rename not atomic) | Detect if possible; warn; still use temp and replace |
| Crash during write | Temp file left; cleaned on next start; original intact |
| Crash between backup and replace | Recovery finds `.bak` and temp; offers restore |

## 11. ZIP read safety

- MVP never extracts to the file system: entries are read into memory.
- Reject: absolute paths, `..` segments, drive letters, backslashes, symlink entries, duplicate names (case-insensitive comparison on all platforms), entries not in the allowed structure, encrypted entries, compression methods other than STORED and DEFLATE.
- Limits (configurable defaults, justified by the large reference score of about 15 to 30 MB in memory and JSON overhead): max entries 10 000, max uncompressed score.json 256 MB, max total uncompressed 2 GB (audio), max compression ratio 100:1 for deflated entries (S5: the large reference score compresses about 17.5:1; a 1 GiB bomb was rejected after 104 MB). Entry and ratio limits apply per entry, with a 4 KB floor on the compressed size so tiny entries are not rejected.
- Limits are enforced on actual bytes read from a bounded reader, not on header-declared sizes.
- ZIP64: accepted for reading assets over 4 GB? Decision: reject over the total limit anyway; ZIP64 headers accepted if within limits [Proposed].
- Use `zip` crate with `default-features = false` and deflate only ([technology-evaluation.md](../research/technology-evaluation.md)).

## 12. Backup and crash recovery

- Autosave: every 2 minutes when dirty (configurable), and on app backgrounding or window close prompts, to `recovery/<project-id>/autosave-<millis>.recovery` using the same atomic write (an internal app-data name, not the product extension). Retain 3 rotating files.
- Session marker: a lock file in app data written at start and removed on clean exit. If present at start, an unclean shutdown happened.
- On start after unclean shutdown: list recovery files newer than their source project and offer Restore, Open original, or Discard.
- Untitled documents autosave to recovery as well.
- Recovery files are deleted after a successful normal save of the source.
- A restored document keeps the original location but starts unsaved (dirty), so it keeps autosaving and its recovery files stay until the next successful save (`DocumentSession::recover`).

Status (Phase 6b-1, [ADR-030](decisions/ADR-030-editor-debt-clipboard-and-session-marker.md)): all of the above is implemented. The app writes the session marker at start and removes it only on a clean exit, after the "save changes?" prompt (Save, Don't save, Cancel) on window close, New, Open and MIDI import. The prompt writes an autosave before it asks, so a system shutdown during the prompt leaves both the autosave and the marker, and the next start offers it. Which entries are offered is decided in the core (`mc_app::recovery::offered`): none after a clean session; otherwise those newer than their source, untitled ones, and ones whose source file is gone. An untouched new document is not prompted for (`DocumentSession::has_unsaved_work`). This closes open points O-1 to O-3 of [phase-3-exit-report.md](../roadmap/phase-3-exit-report.md). No format change: the marker and recovery files are app data outside the package.

## 13. Import and export boundaries

The native loader is the only path that produces a Score from native files. Importers live in adapter modules, use `ScoreBuilder`, and produce reports. Exporters read snapshots and never mutate. No importer writes the native format directly.

## 14. Round-trip test suite (FF-11, FF-32)

1. Every fixture of every released version loads and validates.
2. Load, save, load gives semantic equality.
3. Current version output is byte-stable against golden files.
4. Unknown content (section 9): (4a) an unknown field injected at every DTO level and an unknown extensible-enum variant are each reported with the correct JSON path; (4b) a document with unknown content cannot be saved over its original path and can be saved with Save As; (4c, from format 1.1 with U3) injected content survives load, edit elsewhere, and save.
5. Migration chain output equals expected fixtures for each older version.
6. Newer major version and unknown required features are refused with typed errors.
7. Corrupted hash, truncated ZIP, traversal entries, duplicate entries and decompression bombs produce typed errors and no panic.
8. DTO JSON schema snapshot (schemars) diff fails CI unless the format version changed or the change is marked additive in the changelog.
9. Fuzzing of the loader with cargo-fuzz on Linux CI (Windows libFuzzer support is limited [To verify]).
