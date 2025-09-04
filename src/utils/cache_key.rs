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
pub const CACHE_PREFIX_POST_ID: &str = "post:id:";          // + {uuid}
pub const CACHE_PREFIX_POSTS: &str = "posts:";              // list queries start with this
pub const CACHE_PREFIX_USER_ID: &str = "user:id:";          // + {uuid}
pub const CACHE_PREFIX_USERS: &str = "users:";              // list queries start with this
pub const CACHE_PREFIX_USER_POSTS: &str = "user_posts:user:"; // + {uuid}:...

impl CacheKeyBuilder {
    pub fn new(base: impl Into<String>) -> Self { Self { base: base.into(), segs: Vec::new(), used_labels: std::collections::HashSet::new() } }
    fn push_kv(&mut self, key: &str, val: String) {
        // Enforce uniqueness of labels to prevent accidental overwrites like .kv("page",1).kv("page",2)
        debug_assert!(
            !self.used_labels.contains(key),
            "duplicate cache key segment label detected: {key}"
        );
        self.used_labels.insert(key.to_string());
        self.segs.push(format!("{}:{}", key, val));
    }
    pub fn kv(mut self, key: &str, value: impl std::fmt::Display) -> Self { self.push_kv(key, value.to_string()); self }
    pub fn kv_opt<T: std::fmt::Display>(mut self, key: &str, opt: Option<T>) -> Self {
        match opt { Some(v) => self.push_kv(key, v.to_string()), None => self.push_kv(key, "all".to_string()) }
        self
    }
    pub fn build(self) -> String {
        if self.segs.is_empty() { self.base } else { format!("{}:{}", self.base, self.segs.join(":")) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn builds_expected() {
        let k = CacheKeyBuilder::new("posts").kv("page", 1).kv("limit", 20).kv_opt("status", Option::<String>::None).build();
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
        let _ = CacheKeyBuilder::new("dup").kv("page", 1).kv("page", 2).build();
    }
}