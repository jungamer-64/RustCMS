/// Small helper utilities used by handlers to duplicate cloneable values
/// for use across multiple closures without repeating `.clone()` boilerplate.
///
/// Example:
/// let (a1, a2) = crate::utils::dup::dup(&a);
/// // a1 and a2 are independent owned clones of `a`.
pub fn dup<T: Clone>(v: &T) -> (T, T) {
    (v.clone(), v.clone())
}
