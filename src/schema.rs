// Struct representing the request body for creating a new Todo
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CreateTodoSchema {
    pub task: String,
    pub completed: bool,
}

// Struct representing the request body for updating a Todo
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct UpdateTodoSchema {
    pub task: String,
    pub completed: bool,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SignupSchema {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ConfirmUserSchema {
    pub username: String,
    pub confirmation_code: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct LoginSchema{
    pub username:String,
    pub password:String
}