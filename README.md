# axum-todo-web-app-rust
This Rust code defines a CRUD API using Axum and SQLx, managing Todo items in a SQLite database with JSON serialization.
## Table of Contents

- [Installation](#installation)
- [Usage](#usage)
- [API Endpoints](#api-endpoints)

## Installation

To run this project locally, follow these steps:

```bash
# Clone the repository
git clone https://github.com/pavanmikkilineni/axum-todo-web-app-rust.git

# Change into the project directory
cd axum-todo-web-app-rust

# Build and run the project
cargo run
```

## Usage

Once the project is running, you can interact with the API using tools like curl or your preferred API client. Example:

```bash
# Get all Todo items
curl http://localhost:3000/todos

# Create a new Todo item
curl -X POST -H "Content-Type: application/json" -d '{"task":"New Task","completed":false}' http://localhost:3000/todos

# Get a specific Todo item by ID
curl http://localhost:3000/todos/1

# Update a Todo item by ID
curl -X PATCH -H "Content-Type: application/json" -d '{"task":"Updated Task","completed":true}' http://localhost:3000/todos/1

# Delete a Todo item by ID
curl -X DELETE http://localhost:3000/todos/1
```
## API Endpoints

- `GET /todos`: Get all Todo items.
- `POST /todos`: Create a new Todo item.
- `GET /todos/:id`: Get a specific Todo item by ID.
- `PATCH /todos/:id`: Update a Todo item.
- `DELETE /todos/:id`: Delete a Todo item.
  
Feel free to modify and enhance the code to suit your needs. Happy coding!
