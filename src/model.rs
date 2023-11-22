
// Data model representing a Todo item
#[derive(Debug, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct Todo {
    id: i32,
    task: String,
    completed: bool,
}