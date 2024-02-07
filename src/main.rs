mod handler;
mod middleware;
mod model;
mod route;
mod schema;

use axum::{
    http::{
        header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
        HeaderValue, Method,
    },
    Server,
};

use aws_sdk_cognitoidentityprovider as cognitoidentity;

use cognitoidentity::Client;

use sqlx::{migrate::MigrateDatabase, sqlite::SqlitePoolOptions, Pool, Sqlite};
use tower_http::cors::CorsLayer;

use std::{net::SocketAddr, sync::Arc};

use dotenv::dotenv;

use crate::route::create_router;

// Struct representing the application state
pub struct AppState {
    db: Pool<Sqlite>,
    client: Client,
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
        completed BOOLEAN NOT NULL DEFAULT 0,
        username VARCHAR(120) NOT NULL
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
    let app = create_router(app_state).layer(cors);

    println!("🚀 Server started successfully");

    // Specify the address and port to run the server on
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    // Start the Axum server
    Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
