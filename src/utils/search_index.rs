//! Generic search indexing enum to avoid per-entity safe wrappers where not needed
#![allow(dead_code)]

#[cfg(feature = "search")]
#[derive(Debug)]
pub enum SearchEntity<'a> {
    Post(&'a crate::models::Post),
    User(&'a crate::models::User),
}
