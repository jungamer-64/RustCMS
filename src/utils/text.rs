use regex::Regex;

/// Strip HTML tags and collapse whitespace.
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
pub fn clean_tags(tags: Option<&Vec<String>>) -> Vec<String> {
    match tags {
        Some(v) => {
            let mut seen = std::collections::HashSet::new();
            let mut out = Vec::new();
            for tag in v.iter() {
                let cleaned = tag.trim().to_lowercase();
                if cleaned.len() > 2 && cleaned.len() < 50 && seen.insert(cleaned.clone()) {
                    out.push(cleaned);
                }
            }
            out
        }
        None => Vec::new(),
    }
}

/// Clean categories: take single optional category, trim and lowercase
pub fn clean_categories(cat: Option<&String>) -> Vec<String> {
    cat.as_ref()
        .map(|c| vec![c.trim().to_lowercase()])
        .unwrap_or_default()
}
