use std::sync::Arc;

use axum::{Router, routing::{get, post}, middleware::{from_fn_with_state, from_fn}};

use crate::{handler::*, AppState, middleware::mw_require_auth};

pub fn create_router(app_state:Arc<AppState>)->Router{
    let app = Router::new()
        .route("/", get(health_checker_handler))
        .route("/todos", get(get_todos).post(create_todo))
        .route(
            "/todos/:id",
            get(get_todo).patch(update_todo).delete(delete_todo),
        )
        .route("/login", post(login))
        .route("/signup", post(signup))
        .route("/confirm", post(confirm_user))
        .route_layer(from_fn(mw_require_auth))
        .with_state(app_state);
    app
}