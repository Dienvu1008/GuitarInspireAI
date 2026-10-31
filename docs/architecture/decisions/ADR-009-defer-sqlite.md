# ADR-009 Defer SQLite

Status: Accepted
Date: 2026-10-31
Related documents: [data-architecture.md](../data-architecture.md), [dependency-rules.md](../dependency-rules.md)

## Context
SQLite was proposed for metadata, indexes and caches. The MVP has only preferences and recent files.

## Decision
No SQLite in MVP. Preferences and recent files are JSON. When introduced, SQLite is used from Rust (`rusqlite`, bundled) in a dedicated adapter crate, never for canonical score content.

## Alternatives
SQLite from day one; Dart-side SQLite (sqflite or drift).

## Trade-offs
Less infrastructure now; a later migration of recent-files JSON into an index (trivial).

## Consequences
GR-02 ban is simple: rusqlite is banned everywhere in MVP.

## Related fitness functions
FF-02.

## Reconsideration conditions
Library index over many projects, analysis cache with hash lookups, practice history, or search features.
