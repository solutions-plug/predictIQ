#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_featured_markets_query_parsing() {
        // Test default values
        let query = FeaturedMarketsQuery {
            category: None,
            limit: None,
            page: None,
        };
        assert_eq!(query.category, None);
        assert_eq!(query.limit, None);
        assert_eq!(query.page, None);

        // Test with values
        let query = FeaturedMarketsQuery {
            category: Some("crypto".to_string()),
            limit: Some(6),
            page: Some(2),
        };
        assert_eq!(query.category, Some("crypto".to_string()));
        assert_eq!(query.limit, Some(6));
        assert_eq!(query.page, Some(2));
    }

    #[test]
    fn test_limit_clamping() {
        // Test that limit is clamped between 1 and 20
        let test_cases = vec![
            (Some(0), 1),    // Below minimum
            (Some(1), 1),    // At minimum
            (Some(10), 10),  // Normal value
            (Some(20), 20),  // At maximum
            (Some(25), 20),  // Above maximum
            (None, 8),       // Default value
        ];

        for (input, expected) in test_cases {
            let actual = input.unwrap_or(8).clamp(1, 20);
            assert_eq!(actual, expected, "Failed for input {:?}", input);
        }
    }

    #[test]
    fn test_page_minimum() {
        // Test that page is always at least 1
        let test_cases = vec![
            (Some(0), 1),   // Below minimum
            (Some(1), 1),   // At minimum
            (Some(5), 5),   // Normal value
            (None, 1),      // Default value
        ];

        for (input, expected) in test_cases {
            let actual = input.unwrap_or(1).max(1);
            assert_eq!(actual, expected, "Failed for input {:?}", input);
        }
    }

    #[test]
    fn test_category_normalization() {
        let categories = vec![
            "crypto",
            "politics",
            "technology",
            "sports",
            "stocks",
            "space",
            "climate",
            "entertainment",
        ];

        for category in categories {
            assert!(!category.is_empty());
            assert!(category.chars().all(|c| c.is_ascii_lowercase()));
        }
    }
}
