# Domain

Contains business objects and domain-specific rules.

This placeholder directory is part of an incremental restructure. For now it re-exports
existing `crate::models` so code can `use crate::domain::models::...` during refactorings.

Next steps:

- Add domain newtypes and value objects here.
- Move validated string types or ID NewTypes into `domain::value_objects` when ready.
