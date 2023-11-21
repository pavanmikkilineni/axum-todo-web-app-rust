use axum::{
    extract::{Path, Query, State},
    http::{
        header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
        HeaderValue, Method, StatusCode,
    },
    response::IntoResponse,
    routing::{get, post},
    Json, Router, Server,
};

use aws_sdk_cognitoidentityprovider as cognitoidentity;

use base64::{engine::general_purpose, Engine};
use cognitoidentity::{
    types::{builders::AttributeTypeBuilder, AttributeType},
    Client,
};
use ring::{digest, hmac};
use serde_json::json;

use sqlx::{migrate::MigrateDatabase, query, query_as, sqlite::SqlitePoolOptions, Pool, Sqlite};
use tower_http::cors::CorsLayer;

use std::{net::SocketAddr, sync::Arc};

use dotenv::dotenv;

// Data model representing a Todo item
#[derive(Debug, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
struct Todo {
    id: i32,
    task: String,
    completed: bool,
}

// Struct representing the request body for creating a new Todo
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct CreateTodoSchema {
    task: String,
    completed: bool,
}

// Struct representing the request body for updating a Todo
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct UpdateTodoSchema {
    task: String,
    completed: bool,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct SignupSchema {
    username: String,
    email: String,
    password: String,
}

// Struct representing the application state
pub struct AppState {
    db: Pool<Sqlite>,
    client: Client,
}

// Handler for the health checker route
async fn health_checker_handler() -> impl IntoResponse {
    const MESSAGE: &str = "Simple CRUD API with Rust, SQLX, Postgres, and Axum";

    let json_response = serde_json::json!({
        "status": "success",
        "message": MESSAGE
    });

    Json(json_response)
}

// Handler for getting all Todo items
async fn get_todos(
    State(data): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Fetch all Todo items from the database
    let todos_result = query_as::<_, Todo>("SELECT id, task, completed FROM todos")
        .fetch_all(&data.db)
        .await;
    if todos_result.is_err() {
        // Handle error response if fetching todos fails
        let error_response = serde_json::json!({
            "status": "fail",
            "message": "Something bad happened while fetching all todo items",
        });
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)));
    }

    // Prepare success response with fetched todos
    let todos = todos_result.unwrap();
    let json_response = serde_json::json!({
        "status": "success",
        "results": todos.len(),
        "todos": todos
    });
    Ok((StatusCode::OK, Json(json_response)))
}

// Handler for creating a new Todo
async fn create_todo(
    State(data): State<Arc<AppState>>,
    Json(body): Json<CreateTodoSchema>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Insert a new Todo into the database
    let todo_result = query_as::<_, Todo>(
        "INSERT INTO todos (task, completed) VALUES (?, ?) RETURNING id, task, completed",
    )
    .bind(body.task)
    .bind(body.completed)
    .fetch_one(&data.db)
    .await;

    // Handle the result and prepare the response
    match todo_result {
        Ok(todo) => {
            let todo_response = json!({"status": "success","data": json!({
                "todo": todo
            })});

            Ok((StatusCode::CREATED, Json(todo_response)))
        }
        Err(e) => {
            // Handle specific error cases and prepare error response
            if e.to_string()
                .contains("duplicate key value violates unique constraint")
            {
                let error_response = serde_json::json!({
                    "status": "fail",
                    "message": "Todo with that title already exists",
                });
                Err((StatusCode::CONFLICT, Json(error_response)))
            } else {
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"status": "error","message": format!("{:?}", e)})),
                ))
            }
        }
    }
}

// Handler for getting a specific Todo by ID
async fn get_todo(
    Path(id): Path<i32>,
    State(data): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Fetch a Todo by ID from the database
    let todo_result =
        sqlx::query_as::<_, Todo>("SELECT id, task, completed FROM todos where id = ?")
            .bind(id)
            .fetch_one(&data.db)
            .await;

    // Handle the result and prepare the response
    match todo_result {
        Ok(todo) => {
            let todo_response = serde_json::json!({"status": "success","data": serde_json::json!({
                "todo": todo
            })});

            Ok((StatusCode::OK, Json(todo_response)))
        }
        Err(_) => {
            // Handle the case when the Todo with the specified ID is not found
            let error_response = serde_json::json!({
                "status": "fail",
                "message": format!("Todo with ID: {} not found", id)
            });
            Err((StatusCode::NOT_FOUND, Json(error_response)))
        }
    }
}

// Handler for updating a Todo by ID
async fn update_todo(
    Path(id): Path<i32>,
    State(data): State<Arc<AppState>>,
    Json(body): Json<UpdateTodoSchema>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Update a Todo by ID in the database
    let todo_result = query_as::<_, Todo>(
        "UPDATE todos SET task = ?, completed = ? WHERE id = ? RETURNING id, task, completed",
    )
    .bind(body.task)
    .bind(body.completed)
    .bind(id)
    .fetch_one(&data.db)
    .await;

    // Handle the result and prepare the response
    match todo_result {
        Ok(todo) => {
            let todo_response = serde_json::json!({"status": "success","data": serde_json::json!({
                "todo": todo
            })});

            Ok(Json(todo_response))
        }
        Err(err) => {
            // Handle the case when the update operation fails
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"status": "error","message": format!("{:?}", err)})),
            ))
        }
    }
}

// Handler for deleting a Todo by ID
pub async fn delete_todo(
    Path(id): Path<i32>,
    State(data): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Delete a Todo by ID from the database
    let rows_affected = query("DELETE FROM todos WHERE id = ?")
        .bind(id)
        .execute(&data.db)
        .await
        .unwrap()
        .rows_affected();
    if rows_affected == 0 {
        // Handle the case when the Todo with the specified ID is not found
        let error_response = serde_json::json!({
            "status": "fail",
            "message": format!("Note with ID: {} not found", id)
        });
        return Err((StatusCode::NOT_FOUND, Json(error_response)));
    }

    Ok(StatusCode::NO_CONTENT)
}

async fn login() -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    Ok(())
}

async fn signup(
    State(data): State<Arc<AppState>>,
    Json(body): Json<SignupSchema>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let client_id = std::env::var("CLIENT_ID").unwrap();

    let client_secret = generate_secret_hash(
        &std::env::var("CLIENT_SECRET").unwrap(),
        &body.username,
        &client_id,
    );
    let _user_pool_id = std::env::var("USER_POOL_ID").unwrap();

    let user_attribute_email = AttributeTypeBuilder::default()
        .name("email")
        .value(&body.email)
        .build()
        .unwrap();

    let signup_fluent_builder = data
        .client
        .sign_up()
        .client_id(client_id)
        .secret_hash(client_secret)
        .username(&body.username)
        .password(&body.password)
        .user_attributes(user_attribute_email);

    match signup_fluent_builder.send().await {
        Ok(response) => println!("Response: {:?}",response),
        Err(error) => println!("Error: {:?}",error),
    };

    Ok(())

}

fn generate_secret_hash(client_secret: &str, user_name: &str, client_id: &str) -> String {
    let key = hmac::Key::new(hmac::HMAC_SHA256, client_secret.as_bytes());
    let msg = [user_name.as_bytes(), client_id.as_bytes()].concat();

    let signature = hmac::sign(&key, &msg);

    let encoded_hash = general_purpose::STANDARD.encode(signature.as_ref());

    encoded_hash
}

// Entry point of the application
#[tokio::main]
async fn main() {
    dotenv().ok();

    let config = aws_config::load_from_env().await;
    let client = aws_sdk_cognitoidentityprovider::Client::new(&config);

    const DB_URL: &str = "sqlite://todo.db";

    // Check if the database exists, if not, create it
    if !Sqlite::database_exists(DB_URL).await.unwrap_or(false) {
        println!("Creating database {}", DB_URL);
        match Sqlite::create_database(DB_URL).await {
            Ok(_) => println!("Create db success"),
            Err(error) => panic!("error: {}", error),
        }
    } else {
        println!("Database already exists");
    }

    // Connect to the database
    let pool = match SqlitePoolOptions::new()
        .max_connections(10)
        .connect(DB_URL)
        .await
    {
        Ok(pool) => {
            println!("✅ Connection to the database is successful!");
            pool
        }
        Err(err) => {
            println!("🔥 Failed to connect to the database: {:?}", err);
            std::process::exit(1);
        }
    };

    // Create the 'todos' table if it doesn't exist
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS todos (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        task TEXT NOT NULL,
        completed BOOLEAN NOT NULL DEFAULT 0
    );"#,
    )
    .execute(&pool)
    .await
    .unwrap();

    println!("Create todo table");

    // Create an Arc-wrapped instance of the application state
    let app_state = Arc::new(AppState {
        db: pool.clone(),
        client: client.clone(),
    });

    // Configure CORS settings for the application
    let cors = CorsLayer::new()
        .allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_credentials(true)
        .allow_headers([AUTHORIZATION, ACCEPT, CONTENT_TYPE]);

    // Create the Axum application with routes and middleware
    let app = Router::new()
        .route("/", get(health_checker_handler))
        .route("/todos", get(get_todos).post(create_todo))
        .route(
            "/todos/:id",
            get(get_todo).patch(update_todo).delete(delete_todo),
        )
        .route("/login", post(login))
        .route("/signup", post(signup))
        .with_state(app_state)
        .layer(cors);

    println!("🚀 Server started successfully");

    // Specify the address and port to run the server on
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    // Start the Axum server
    Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
