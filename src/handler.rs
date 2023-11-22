use std::sync::Arc;

use aws_sdk_cognitoidentityprovider::types::builders::AttributeTypeBuilder;
use aws_sdk_cognitoidentityprovider::types::AuthFlowType::UserPasswordAuth;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use base64::{engine::general_purpose, Engine};
use ring::hmac;
use serde_json::json;
use sqlx::{query, query_as};

use crate::{
    model::Todo,
    schema::{ConfirmUserSchema, CreateTodoSchema, SignupSchema, UpdateTodoSchema, LoginSchema},
    AppState,
};

// Handler for the health checker route
pub async fn health_checker_handler() -> impl IntoResponse {
    const MESSAGE: &str = "Simple CRUD API with Rust, SQLX, Postgres, and Axum";

    let json_response = serde_json::json!({
        "status": "success",
        "message": MESSAGE
    });

    Json(json_response)
}

// Handler for getting all Todo items
pub async fn get_todos(
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
pub async fn create_todo(
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
pub async fn get_todo(
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
pub async fn update_todo(
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
            "status": "error",
            "message": format!("Note with ID: {} not found", id)
        });
        return Err((StatusCode::NOT_FOUND, Json(error_response)));
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn login(
    State(data): State<Arc<AppState>>,
    Json(body): Json<LoginSchema>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {

    let client_id = std::env::var("CLIENT_ID").unwrap();

    let client_secret = generate_secret_hash(
        &std::env::var("CLIENT_SECRET").unwrap(),
        &body.username,
        &client_id,
    );

    let _user_pool_id = std::env::var("USER_POOL_ID").unwrap();

    let initiate_auth_fluent_builder = data.client.initiate_auth()
    .client_id(client_id)
    .auth_flow(UserPasswordAuth)
    .auth_parameters("USERNAME",&body.username)
    .auth_parameters("PASSWORD", &body.password)
    .auth_parameters("SECRET_HASH", client_secret);
    
    match initiate_auth_fluent_builder.send().await{
        Ok(response) => {
            let access_token = response.authentication_result().unwrap().access_token().unwrap();
            let id_token = response.authentication_result().unwrap().id_token().unwrap();
            let refresh_token = response.authentication_result().unwrap().refresh_token().unwrap();
             let success_response = serde_json::json!({"status": "success","data": serde_json::json!({
                "access_token": access_token,
                "id_token":id_token,
                "refresh_token":refresh_token
            })});
            Ok((StatusCode::OK,Json(success_response)))
        },
        Err(error) => {
            let error_response = serde_json::json!({
                "status": "error","message": format!("{:?}", error)
            });
            Err((StatusCode::OK,Json(error_response)))
        },
    }

}

pub async fn signup(
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
        Ok(response) => {
            let success_response = if response.user_confirmed {
                serde_json::json!({
                    "status": "success","message": "User requires confirmation. Check email for a verification code."
                })
            } else {
                serde_json::json!({
                    "status": "success","message": "User is confirmed and ready to use."
                })
            };

            Ok((StatusCode::CREATED, Json(success_response)))
        }
        Err(error) => {
            let error_response = serde_json::json!({
                "status": "error","message": format!("{}",error.to_string())
            });
            Err((StatusCode::BAD_REQUEST, Json(error_response)))
        }
    }
}

pub async fn confirm_user(
    State(data): State<Arc<AppState>>,
    Json(body): Json<ConfirmUserSchema>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let client_id = std::env::var("CLIENT_ID").unwrap();

    let client_secret = generate_secret_hash(
        &std::env::var("CLIENT_SECRET").unwrap(),
        &body.username,
        &client_id,
    );

    let confirm_signup_fluent_builder = data
        .client
        .confirm_sign_up()
        .client_id(client_id)
        .secret_hash(client_secret)
        .username(&body.username)
        .confirmation_code(&body.confirmation_code);

    match confirm_signup_fluent_builder.send().await {
        Ok(_) => {
            let success_response = serde_json::json!({
                "status": "success","message": "User is confirmed and ready to use."
            });
            Ok((StatusCode::OK, Json(success_response)))
        }
        Err(error) => {
            let error_response = serde_json::json!({
                "status": "error","message": format!("{}",error.to_string())
            });
            Err((StatusCode::OK, Json(error_response)))
        }
    }
}

fn generate_secret_hash(client_secret: &str, user_name: &str, client_id: &str) -> String {
    let key = hmac::Key::new(hmac::HMAC_SHA256, client_secret.as_bytes());
    let msg = [user_name.as_bytes(), client_id.as_bytes()].concat();

    let signature = hmac::sign(&key, &msg);

    let encoded_hash = general_purpose::STANDARD.encode(signature.as_ref());

    encoded_hash
}
