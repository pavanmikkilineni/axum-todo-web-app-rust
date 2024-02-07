
// Data model representing a Todo item
#[derive(Debug, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct Todo {
    pub(crate) id: i32,
    pub(crate) task: String,
    pub(crate) completed: bool,
}

#[derive(Debug,Clone)]
pub struct CurrentUser{
    pub(crate) username:String
}