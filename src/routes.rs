use crate::controller;
use crate::state::AppState;
use axum::Router;
use axum::routing::{get, patch, post};
use std::sync::Arc;

pub fn create_app(state: Arc<AppState>) -> Router {
    let api_router = Router::new()
        // ORDER
        .route("/orders", post(controller::order::checkout))
        .route("/orders/:order_id", get(controller::order::get_order))
        .route(
            "/orders/:order_id/payment",
            patch(controller::order::payment),
        )
        .route(
            "/orders/:order_id/confirm",
            patch(controller::order::confirm_order),
        )
        .route(
            "/orders/:order_id/purchased",
            patch(controller::order::purchased),
        )
        .route(
            "/orders/:order_id/shipped",
            patch(controller::order::shipped),
        )
        .route(
            "/orders/:order_id/history",
            get(controller::order::get_order_history),
        )
        .route(
            "/orders/:order_id/cancel",
            post(controller::order::cancel_order),
        )
        .route("/orders/my/purchases", get(controller::order::my_purchases))
        .route("/orders/my/sales", get(controller::order::my_sales))
        // RATING
        .route(
            "/orders/:order_id/rating/jastiper",
            get(controller::rating_jastiper::get_rating),
        )
        .route(
            "/orders/:order_id/rating/jastiper",
            post(controller::rating_jastiper::submit_rating_jastiper),
        )
        .route(
            "/orders/:order_id/rating/product",
            get(controller::rating_product::get_rating),
        )
        .route(
            "/orders/:order_id/rating/product",
            post(controller::rating_product::submit_rating_product),
        )
        // INTERNAL
        .route(
            "/internal/orders/:order_id/payment-info",
            get(controller::internal::payment_info),
        )
        .route(
            "/internal/orders/:order_id/payment-confirmed",
            post(controller::internal::payment_confirmed),
        )
        .route(
            "/internal/orders/:order_id/refund-confirmed",
            post(controller::internal::refund_confirmed),
        )
        // ADMIN
        .route("/admin/orders", get(controller::admin::get_all))
        .route("/admin/orders/:order_id", get(controller::admin::get_order))
        .route(
            "/admin/orders/:order_id/force-cancel",
            post(controller::admin::force_cancel),
        )
        .with_state(state.clone());

    Router::new().merge(api_router)
}
