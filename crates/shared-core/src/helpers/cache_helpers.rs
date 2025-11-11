//! Cache helper utilities shared across binaries.
//!
//! Provides cache key builders and standardized list key encoders so each
//! service does not have to reinvent ad-hoc `format!` strings.

use std::collections::HashSet;

/// Cache TTL constants (seconds)
pub const CACHE_TTL_SHORT: u64 = 120;
pub const CACHE_TTL_DEFAULT: u64 = 300;
pub const CACHE_TTL_LONG: u64 = 600;

/// Helper to build colon separated cache keys with labeled segments.
#[derive(Default)]
pub struct CacheKeyBuilder {
    base: String,
    segments: Vec<String>,
    used_labels: HashSet<String>,
}

impl CacheKeyBuilder {
    #[must_use]
    pub fn new(base: impl Into<String>) -> Self {
        Self {
            base: base.into(),
            segments: Vec::new(),
            used_labels: HashSet::new(),
        }
    }

    fn push_kv(&mut self, key: &str, value: &str) {
        debug_assert!(
            !self.used_labels.contains(key),
            "duplicate cache key segment label detected: {key}"
        );
        self.used_labels.insert(key.to_string());
        self.segments.push(format!("{key}:{value}"));
    }

    #[must_use]
    pub fn kv(mut self, key: &str, value: impl std::fmt::Display) -> Self {
        self.push_kv(key, &value.to_string());
        self
    }

    #[must_use]
    pub fn kv_opt<T: std::fmt::Display>(mut self, key: &str, value: Option<T>) -> Self {
        match value {
            Some(v) => self.push_kv(key, &v.to_string()),
            None => self.push_kv(key, "all"),
        }
        self
    }

    #[must_use]
    pub fn build(self) -> String {
        if self.segments.is_empty() {
            self.base
        } else {
            let segs = self.segments.join(":");
            format!("{}:{segs}", self.base)
        }
    }
}

/// Convenience helper for entity id cache keys (`prefix:id:{uuid}`).
#[must_use]
pub fn entity_id_key(prefix: &str, id: impl std::fmt::Display) -> String {
    format!("{prefix}:id:{id}")
}

/// Enum describing standardized list cache keys.
pub enum ListCacheKey<'a> {
    Posts {
        page: u32,
        limit: u32,
        status: &'a Option<String>,
        author: &'a Option<uuid::Uuid>,
        tag: &'a Option<String>,
        sort: &'a Option<String>,
    },
    Users {
        page: u32,
        limit: u32,
        role: &'a Option<String>,
        active: Option<bool>,
        sort: &'a Option<String>,
    },
}

impl ListCacheKey<'_> {
    #[must_use]
    pub fn to_cache_key(&self) -> String {
        match self {
            ListCacheKey::Posts {
                page,
                limit,
                status,
                author,
                tag,
                sort,
            } => build_list_cache_key(
                "posts",
                *page,
                *limit,
                &[
                    ("status", (*status).clone()),
                    ("author", author.map(|u| u.to_string())),
                    ("tag", (*tag).clone()),
                    ("sort", (*sort).clone()),
                ],
            ),
            ListCacheKey::Users {
                page,
                limit,
                role,
                active,
                sort,
            } => build_list_cache_key(
                "users",
                *page,
                *limit,
                &[
                    ("role", (*role).clone()),
                    ("active", active.map(|b| b.to_string())),
                    ("sort", (*sort).clone()),
                ],
            ),
        }
    }
}

/// Helper to build standard list keys: `{base}:page:{page}:limit:{limit}:...`.
#[must_use]
pub fn build_list_cache_key(
    base: &str,
    page: u32,
    limit: u32,
    pairs: &[(&str, Option<String>)],
) -> String {
    let mut builder = CacheKeyBuilder::new(base)
        .kv("page", page)
        .kv("limit", limit);

    for (label, value) in pairs {
        builder = builder.kv_opt(label, value.clone());
    }

    builder.build()
}
