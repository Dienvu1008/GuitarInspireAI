# Glossary

Purpose: Shared vocabulary for music, guitar and architecture terms used across the package.
Status: Accepted (living document).
Related documents: [docs/README.md](README.md), [bounded-contexts.md](architecture/bounded-contexts.md), [domain-invariants.md](architecture/domain-invariants.md).

## Status and fact labels

| Label | Meaning |
|---|---|
| Accepted | Decision in force. Change requires an ADR. |
| Proposed | Recommended, awaiting a spike result or user confirmation. |
| Deferred | Intentionally postponed, with a reconsideration trigger. |
| [Verified: source] | Checked against an official source during planning. |
| [Assumption] | Believed true, not checked. Must not be relied on without validation. |
| [To verify] | Must be checked before the dependent decision is finalized. |

## Music and guitar terms

| Term | Definition |
|---|---|
| Score | The complete musical content of a project: master bars, tracks and their content. |
| Master bar | Song-level bar properties shared by all tracks: time signature, key, repeats, markers. |
| Bar (measure) | One track's content for one master bar. |
| Voice | Independent rhythmic line inside a bar. A bar has one or more voices. |
| Beat | A single attack event in a voice with one duration. Holds zero (rest) or more notes. |
| Note | One sounding element in a beat. For fretted tracks: a string and fret. |
| Duration | Rational fraction of a whole note, including dots and tuplets. Never a float. |
| Tick | Integer timeline unit derived from rational time for playback (resolution fixed per timeline). |
| Spelled pitch | Letter, alteration and octave, for example F#4 vs Gb4. Not limited to MIDI range. |
| Pitch class | Pitch modulo octave, 0 to 11. |
| MIDI note | Integer 0 to 127. A playback and interchange type, not a theory type. |
| Tuning | Ordered open-string pitches of a fretted instrument. |
| String index | Physical position of a string, 1-based. String 1 is the highest-pitched string of the standard layout. Stable under retuning; never renumbered by pitch. See [bounded-contexts.md](architecture/bounded-contexts.md). |
| Effective bar duration | Playback length of a master bar: the maximum of its nominal length and the longest voice across all tracks. See [domain-invariants.md](architecture/domain-invariants.md) section 4a. |
| Fret | Integer fret number, 0 means open string. |
| Capo | Fret offset applied to all strings. |
| Voicing | A concrete set of string and fret positions realizing a chord. At most one position per string. |
| Shape text | Chord-chart notation of a voicing, lowest-pitched string (string 6) first, `x` for a muted string: `x32010` is open C. Frets above 9 use separators: `x-3-5-5-5-3`. |
| Chord quality | Interval structure of a chord (major, m7, m7b5 ...). Display symbols and aliases (`Δ7`, `ø7`, `°`, `+`, `min`) only spell a quality; they are not separate concepts. |
| Slash chord | A chord whose lowest note is not its root, written `C/E` (chord / bass note). |
| Omitted fifth | A perfect fifth left out of a voicing. Allowed by default for chord identification and search; never for dim, aug, m7b5, dim7 or sus chords. |
| Fret span | Highest minus lowest fretted fret plus one. Open strings do not count. |
| Barre | One finger pressing several adjacent strings at the same fret; counts as one fretting finger. |
| CAGED | A system relating five open chord shapes (C, A, G, E, D) to positions on the neck. |
| Pattern | A reusable rhythmic and string-selection template that binds to a voicing. |
| Overfull / underfull bar | A voice whose summed durations are more / less than the time signature length. |
| Diagnostic | A reported, non-blocking well-formedness issue in a committed document. |

## Architecture terms

| Term | Definition |
|---|---|
| Music Core | The Rust Cargo workspace containing domain, application and adapter crates. |
| Shared kernel | `mc-theory`: value objects and pure arithmetic shared by several contexts. |
| Document session | Application-layer owner of one open project: score, undo stacks, revision, event sequence. |
| Command | Typed request to change a document. Validated by the domain. |
| Query / projection | Read-only view model computed from the score, for example TabLayout. |
| Domain event | Fact returned by an aggregate method describing what changed. |
| Application event | Fact produced by the application layer, for example DocumentSaved. |
| Event envelope | Event plus seq, document id, revision, correlation and causation ids. |
| Control plane | Low-rate commands and status between UI and the audio engine. |
| Data plane | Real-time audio sample processing inside audio callbacks. |
| Port / adapter | Interface defined by `mc-app` / implementation of it in an infrastructure crate. |
| Fitness function | Automated check that an architecture rule still holds. |
| Candidate | An analysis or AI result that is not yet user-confirmed musical data. |
| Provenance | Record of how a candidate was produced: source, algorithm or model, version, parameters, confidence. |
| frb | flutter_rust_bridge, the Dart and Rust binding generator. |
| RT | Real-time, as in the audio callback thread. |
| FTE week | One full-time-equivalent week of work by one developer. |
