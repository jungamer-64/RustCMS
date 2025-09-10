//! Cache key builder utility to avoid ad-hoc format! duplication.
//! Produces colon separated stable keys: namespace + labeled segments.
//! Example: CacheKeyBuilder::new("posts")
//!   .kv("page", 1).kv("limit", 20).kv_opt("status", None)
//!   .build() => "posts:page:1:limit:20:status:all".

pub struct CacheKeyBuilder {
    base: String,
    segs: Vec<String>,
    // keep track of label parts to avoid accidental duplicates (debug only enforcement)
    used_labels: std::collections::HashSet<String>,
}

// Common cache key base & prefixes (keep centralized for invalidation & consistency)
pub const CACHE_PREFIX_POST_ID: &str = "post:id:"; // + {uuid}
pub const CACHE_PREFIX_POSTS: &str = "posts:"; // list queries start with this
pub const CACHE_PREFIX_USER_ID: &str = "user:id:"; // + {uuid}
pub const CACHE_PREFIX_USERS: &str = "users:"; // list queries start with this
pub const CACHE_PREFIX_USER_POSTS: &str = "user_posts:user:"; // + {uuid}:...

/// Convenience helper to build a simple entity id based cache key.
/// Example: entity_id_key("post", uuid) => "post:id:{uuid}"
pub fn entity_id_key(prefix: &str, id: impl std::fmt::Display) -> String {
    format!("{}:id:{}", prefix, id)
}

/// Enum describing list style cache keys we standardize on. This avoids a
/// proliferation of small one-off wrapper functions like build_posts_cache_key
/// or build_users_cache_key. Handlers can format through `to_cache_key`.
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

impl<'a> ListCacheKey<'a> {
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

impl CacheKeyBuilder {
    pub fn new(base: impl Into<String>) -> Self {
        Self {
            base: base.into(),
            segs: Vec::new(),
            used_labels: std::collections::HashSet::new(),
        }
    }
    fn push_kv(&mut self, key: &str, val: String) {
        // Enforce uniqueness of labels to prevent accidental overwrites like .kv("page",1).kv("page",2)
        debug_assert!(
            !self.used_labels.contains(key),
            "duplicate cache key segment label detected: {key}"
        );
        self.used_labels.insert(key.to_string());
        self.segs.push(format!("{}:{}", key, val));
    }
    pub fn kv(mut self, key: &str, value: impl std::fmt::Display) -> Self {
        self.push_kv(key, value.to_string());
        self
    }
    pub fn kv_opt<T: std::fmt::Display>(mut self, key: &str, opt: Option<T>) -> Self {
        match opt {
            Some(v) => self.push_kv(key, v.to_string()),
            None => self.push_kv(key, "all".to_string()),
        }
        self
    }
    pub fn build(self) -> String {
        if self.segs.is_empty() {
            self.base
        } else {
            format!("{}:{}", self.base, self.segs.join(":"))
        }
    }
}

/// Helper to build a common list-style cache key used by many handlers.
///
/// Accepts a base (e.g. "posts" or "users"), page/limit and a slice of
/// (label, Option<String>) pairs for additional labeled segments. When the
/// Option is None the key will encode the segment as "label:all" using
/// `kv_opt` semantics.
pub fn build_list_cache_key(
    base: &str,
    page: u32,
    limit: u32,
    pairs: &[(&str, Option<String>)],
) -> String {
    let mut b = CacheKeyBuilder::new(base)
        .kv("page", page)
        .kv("limit", limit);
    for (k, v) in pairs {
        match v {
            Some(val) => {
                b = b.kv(k, val);
            }
            None => {
                b = b.kv_opt::<String>(k, None);
            }
        }
    }
    b.build()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn builds_expected() {
        let k = CacheKeyBuilder::new("posts")
            .kv("page", 1)
            .kv("limit", 20)
            .kv_opt("status", Option::<String>::None)
            .build();
        assert_eq!(k, "posts:page:1:limit:20:status:all");
    }

    #[test]
    fn ordering_is_stable() {
        let a = CacheKeyBuilder::new("x").kv("b", 2).kv("a", 1).build();
        assert_eq!(a, "x:b:2:a:1"); // insertion order preserved
    }

    #[test]
    fn unicode_values_supported() {
        let k = CacheKeyBuilder::new("tag").kv("名前", "値").build();
        assert_eq!(k, "tag:名前:値");
    }

    #[test]
    #[should_panic(expected = "duplicate cache key segment label detected")]
    fn duplicate_label_panics_in_debug() {
        let _ = CacheKeyBuilder::new("dup")
            .kv("page", 1)
            .kv("page", 2)
            .build();
    }
}
