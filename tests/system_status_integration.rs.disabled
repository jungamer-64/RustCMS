use cms_backend::utils::bin_utils::render_health_table_components;

#[test]
fn integration_render_health_table() {
    let overall = "healthy";
    let db = ("up", 11.11_f64, Some(""));
    let cache = ("up", 2.22_f64, None::<&str>);
    let search = ("degraded", 33.33_f64, Some("timeout"));
    let auth = ("up", 4.44_f64, None::<&str>);

    let table = render_health_table_components(overall, db, cache, search, auth);
    let s = format!("{table}");

    assert!(s.contains("Overall"));
    assert!(s.contains("Database"));
    assert!(s.contains("Search"));
    assert!(s.contains("degraded") || s.contains("degrad"));
}
