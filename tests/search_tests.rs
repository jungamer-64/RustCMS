//! Search Module Integration Tests
//!
//! Tests for full-text search functionality using Tantivy

#[cfg(feature = "search")]
mod search_tests {
    use cms_backend::search::*;

    #[test]
    fn search_error_conversion() {
        let err = SearchError::Index("test error".to_string());
        let app_err: cms_backend::AppError = err.into();
        assert!(app_err.to_string().contains("test error"));
    }

    #[test]
    fn search_request_deserialization() {
        let json = r#"{
            "query": "rust programming",
            "limit": 10,
            "offset": 0
        }"#;

        let req: SearchRequest = serde_json::from_str(json).expect("deserialize failed");
        assert_eq!(req.query, "rust programming");
        assert_eq!(req.limit, Some(10));
        assert_eq!(req.offset, Some(0));
    }

    #[test]
    fn search_request_with_filters() {
        let json = r#"{
            "query": "test",
            "filters": [
                {
                    "field": "status",
                    "value": "published",
                    "operator": "equals"
                }
            ]
        }"#;

        let req: SearchRequest = serde_json::from_str(json).expect("deserialize failed");
        assert!(req.filters.is_some());
        let filters = req.filters.unwrap();
        assert_eq!(filters.len(), 1);
        assert_eq!(filters[0].field, "status");
    }

    #[test]
    fn search_request_with_facets() {
        let json = r#"{
            "query": "test",
            "facets": ["category", "author"]
        }"#;

        let req: SearchRequest = serde_json::from_str(json).expect("deserialize failed");
        assert!(req.facets.is_some());
        let facets = req.facets.unwrap();
        assert_eq!(facets.len(), 2);
        assert!(facets.contains(&"category".to_string()));
    }

    #[test]
    fn search_request_with_sort() {
        let json = r#"{
            "query": "test",
            "sort_by": "created_at",
            "sort_order": "desc"
        }"#;

        let req: SearchRequest = serde_json::from_str(json).expect("deserialize failed");
        assert_eq!(req.sort_by, Some("created_at".to_string()));
        assert!(matches!(req.sort_order, Some(SortOrder::Desc)));
    }

    #[test]
    fn search_results_structure() {
        let results = SearchResults::<String> {
            hits: vec!["result1".to_string(), "result2".to_string()],
            total: 2,
            took_ms: 15,
            facets: vec![],
        };

        assert_eq!(results.hits.len(), 2);
        assert_eq!(results.total, 2);
        assert_eq!(results.took_ms, 15);
    }

    #[test]
    fn search_facet_structure() {
        let facet = SearchFacet {
            field: "category".to_string(),
            values: vec![
                FacetValue {
                    value: "tech".to_string(),
                    count: 10,
                },
                FacetValue {
                    value: "news".to_string(),
                    count: 5,
                },
            ],
        };

        assert_eq!(facet.field, "category");
        assert_eq!(facet.values.len(), 2);
        assert_eq!(facet.values[0].count, 10);
    }

    #[test]
    fn filter_operator_deserialization() {
        let json_eq = r#""equals""#;
        let op: FilterOperator = serde_json::from_str(json_eq).expect("deserialize failed");
        assert!(matches!(op, FilterOperator::Equals));

        let json_contains = r#""contains""#;
        let op: FilterOperator = serde_json::from_str(json_contains).expect("deserialize failed");
        assert!(matches!(op, FilterOperator::Contains));

        let json_gt = r#""greaterthan""#;
        let op: FilterOperator = serde_json::from_str(json_gt).expect("deserialize failed");
        assert!(matches!(op, FilterOperator::GreaterThan));

        let json_lt = "\"lessthan\"";
        let op: FilterOperator = serde_json::from_str(json_lt).expect("deserialize failed");
        assert!(matches!(op, FilterOperator::LessThan));
    }

    #[test]
    fn search_filter_structure() {
        let json = r#"{
            "field": "price",
            "value": "100",
            "operator": "greaterthan"
        }"#;

        let filter: SearchFilter = serde_json::from_str(json).expect("deserialize failed");
        assert_eq!(filter.field, "price");
        assert_eq!(filter.value, "100");
        assert!(matches!(filter.operator, FilterOperator::GreaterThan));
    }

    #[test]
    fn search_results_serialization() {
        let results = SearchResults {
            hits: vec!["item1".to_string(), "item2".to_string()],
            total: 2,
            took_ms: 25,
            facets: vec![SearchFacet {
                field: "type".to_string(),
                values: vec![FacetValue {
                    value: "article".to_string(),
                    count: 2,
                }],
            }],
        };

        let json = serde_json::to_string(&results).expect("serialize failed");
        assert!(json.contains("item1"));
        assert!(json.contains("\"total\":2"));
        assert!(json.contains("\"took_ms\":25"));
    }

    #[test]
    fn empty_search_results() {
        let results = SearchResults::<String> {
            hits: vec![],
            total: 0,
            took_ms: 5,
            facets: vec![],
        };

        assert!(results.hits.is_empty());
        assert_eq!(results.total, 0);
    }

    #[test]
    fn search_request_minimal() {
        let json = r#"{"query": "test"}"#;
        let req: SearchRequest = serde_json::from_str(json).expect("deserialize failed");

        assert_eq!(req.query, "test");
        assert!(req.filters.is_none());
        assert!(req.facets.is_none());
        assert!(req.limit.is_none());
        assert!(req.offset.is_none());
    }

    #[test]
    fn search_request_pagination() {
        let json = r#"{
            "query": "test",
            "limit": 20,
            "offset": 40
        }"#;

        let req: SearchRequest = serde_json::from_str(json).expect("deserialize failed");
        assert_eq!(req.limit, Some(20));
        assert_eq!(req.offset, Some(40));
    }

    #[test]
    fn facet_value_serialization() {
        let value = FacetValue {
            value: "test_value".to_string(),
            count: 42,
        };

        let json = serde_json::to_string(&value).expect("serialize failed");
        assert!(json.contains("test_value"));
        assert!(json.contains("42"));
    }
}

#[cfg(not(feature = "search"))]
#[test]
fn search_feature_disabled() {
    // When search feature is disabled, this test ensures the module
    // doesn't break the build
    assert!(true, "search feature is disabled");
}
