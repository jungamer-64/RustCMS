# Infrastructure

Hosts concrete implementations: database adapters, cache adapters, search index code, repositories.

This placeholder directory re-exports existing crates so callers can start using
`crate::infrastructure::repositories` or `crate::infrastructure::database`.

Next steps:

- When ready, move repository implementations and DB adapters into this folder.
- Keep interfaces (traits) in `src/repositories` or `src/domain` as appropriate.
