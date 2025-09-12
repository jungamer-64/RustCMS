use regex::Regex;

/// Strip HTML tags and collapse whitespace.
///
/// # Panics
/// 正規表現のコンパイルに失敗した場合にパニックします（固定のリテラルのため通常は発生しません）。
#[must_use]
pub fn strip_html(content: &str) -> String {
    // Basic HTML tag removal - keep the simple implementation used by models
    // but centralize it so other modules can reuse it.
    let tag_regex = Regex::new(r"<[^>]*>").unwrap();
    tag_regex
        .replace_all(content, " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Clean tags: trim, lowercase, length checks and deduplicate preserving order
#[must_use]
pub fn clean_tags(tags: Option<&Vec<String>>) -> Vec<String> {
    tags.map_or_else(Vec::new, |v| {
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for tag in v {
            let cleaned = tag.trim().to_lowercase();
            if cleaned.len() > 2 && cleaned.len() < 50 && seen.insert(cleaned.clone()) {
                out.push(cleaned);
            }
        }
        out
    })
}

/// Clean categories: take single optional category, trim and lowercase
#[must_use]
pub fn clean_categories(cat: Option<&String>) -> Vec<String> {
    cat.as_ref()
        .map(|c| vec![c.trim().to_lowercase()])
        .unwrap_or_default()
}
