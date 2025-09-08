pub fn contains_case_insensitive(vec: &Vec<String>, item: &str) -> bool {
    let target = item.trim().to_lowercase();
    vec.iter().any(|v| v.trim().to_lowercase() == target)
}
