use std::sync::Arc;

use axum::{
    middleware::from_fn,
    routing::{get, post},
    Router,
};

use crate::{handler::*, middleware::mw_require_auth, AppState};

pub fn create_router(app_state: Arc<AppState>) -> Router {
    let app = Router::new()
        .route("/todos", get(get_todos).post(create_todo))
        .route(
            "/todos/:id",
            get(get_todo).patch(update_todo).delete(delete_todo),
        )
        .route("/logout", post(logout))
        .route_layer(from_fn(mw_require_auth))
        .route("/login", post(login))
        .route("/signup", post(signup))
        .route("/confirm", post(confirm_user))
        .route("/", get(health_checker_handler))
        .with_state(app_state);
    app
}
