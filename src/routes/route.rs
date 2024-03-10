use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};

use crate::{
    handlers::handler::{
        create_note_handler, delete_note_handler, edit_note_handler, get_note_handler,
        note_list_handler, get_name, get_notes_handler,
    },
    AppState,
};

pub fn create_router(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/notes", post(create_note_handler))
        .route("/api/notes", get(note_list_handler))
        .route("/api/notes2", post(get_notes_handler))
        .route(
            "/api/notes/:id",
            get(get_note_handler)
                .patch(edit_note_handler)
                .delete(delete_note_handler),
        )
        .route("/api/:name", get(get_name))
        .with_state(app_state)
}
