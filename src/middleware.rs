use axum::{
    http::{self, Request, StatusCode},
    middleware::Next,
    response::Response,
};

use jsonwebtokens_cognito::KeySet;
use serde_json::Value;

use crate::model::CurrentUser;



pub async fn mw_require_auth<B>(
    mut request: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    let auth_header = request
        .headers()
        .get(http::header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    let auth_header = if let Some(auth_header) = auth_header {
        auth_header
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    let user_pool_region = std::env::var("USER_POOL_REGION").unwrap();
    let user_pool_id = std::env::var("USER_POOL_ID").unwrap();
    let client_id = std::env::var("CLIENT_ID").unwrap();

    let keyset = KeySet::new(user_pool_region, user_pool_id).unwrap();
    let verifier = keyset
        .new_access_token_verifier(&[&client_id])
        .build()
        .unwrap();

    match keyset.verify(&auth_header, &verifier).await {
        Ok(result) => {
            // Match on the Value to ensure it's an object with the "username" field
            if let Value::Object(obj) = result {
                if let Some(username_value) = obj.get("username") {
                    if let Value::String(username) = username_value {
                        // Now you have the username string
                        request.extensions_mut().insert(CurrentUser{
                            username:username.to_string()
                        });
                    } else {
                        // Handle the case where "username" is not a string
                        println!("Username is not a string");
                    }
                } else {
                    // Handle the case where the "username" field is missing
                    println!("Username field is missing");
                }
            } else {
                // Handle the case where the value is not an object
                println!("Value is not an object");
            }
        }
        Err(_) => return Err(StatusCode::UNAUTHORIZED),
    }

    Ok(next.run(request).await)
}

