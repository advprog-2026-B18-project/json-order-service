#[cfg(test)]
mod tests {
    use crate::models::filter_pagination::{OrderFilter, PaginationParams, SortOrder};

    #[test]
    fn sort_order_default_is_asc() {
        let order = SortOrder::default();
        assert!(matches!(order, SortOrder::Asc));
    }

    #[test]
    fn sort_order_clone() {
        let asc = SortOrder::Asc;
        let cloned = asc.clone();
        assert!(matches!(cloned, SortOrder::Asc));

        let desc = SortOrder::Desc;
        let cloned_desc = desc.clone();
        assert!(matches!(cloned_desc, SortOrder::Desc));
    }

    #[test]
    fn pagination_params_default_all_none() {
        let p = PaginationParams::default();
        assert!(p.page.is_none());
        assert!(p.limit.is_none());
        assert!(p.sort_by.is_none());
        assert!(p.order.is_none());
    }

    #[test]
    fn order_filter_default_ids_none() {
        let f = OrderFilter::default();
        assert!(f.status.is_none());
        assert!(f.jastiper_id.is_none());
        assert!(f.titipers_id.is_none());
        assert!(f.product_id.is_none());
    }

    #[test]
    fn pagination_params_page_and_limit() {
        let p = PaginationParams {
            page: Some(2),
            limit: Some(20),
            sort_by: Some("created_at".to_string()),
            order: Some(SortOrder::Desc),
        };
        assert_eq!(p.page, Some(2));
        assert_eq!(p.limit, Some(20));
        assert_eq!(p.sort_by.as_deref(), Some("created_at"));
        assert!(matches!(p.order, Some(SortOrder::Desc)));
    }
}
