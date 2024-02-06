use axum::{
    http::{self, Request, StatusCode},
    middleware::Next,
    response::Response,
};

use jsonwebtoken::{decode, Algorithm, DecodingKey, TokenData, Validation};
use jsonwebtokens_cognito::KeySet;
use reqwest;
use serde::{de::value, Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    // Define the claims in your JWT
    username: String,
    // Add other fields as needed
}

#[derive(Debug, Serialize, Deserialize)]
struct JWK {
    kid: String,
    alg: String,
    kty: String,
    e: String,
    n: String,
    use_: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct JWKS {
    keys: Vec<JWK>,
}

pub async fn mw_require_auth<B>(
    request: Request<B>,
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

    println!("{}", auth_header);

    let user_pool_region = std::env::var("USER_POOL_REGION").unwrap();
    let user_pool_id = std::env::var("USER_POOL_ID").unwrap();
    let client_id = std::env::var("CLIENT_ID").unwrap();

    // match verify_cognito_jwt_token(auth_header, &user_pool_region, &user_pool_id).await {
    //     Ok(token_data) => println!("JWT token is valid! User ID: {}", token_data.claims.username),
    //     Err(_) => return Err(StatusCode::UNAUTHORIZED),
    // }

    let keyset = KeySet::new(user_pool_region, user_pool_id).unwrap();
    let verifier = keyset
        .new_access_token_verifier(&[&client_id])
        .string_equals("my_claim", "foo")
        .build().unwrap();

    match keyset.verify(&auth_header, &verifier).await{
        Ok(result) => println!("{:?}",result),
        Err(_) => return Err(StatusCode::UNAUTHORIZED),
    }

    Ok(next.run(request).await)
}

async fn fetch_jwks(user_pool_region: &str, user_pool_id: &str) -> Result<String, reqwest::Error> {
    let jwks_url = format!(
        "https://cognito-idp.{}.amazonaws.com/{}/.well-known/jwks.json",
        user_pool_region, user_pool_id
    );

    let response = reqwest::get(&jwks_url).await?;
    let jwks = response.text().await?;

    Ok(jwks)
}

fn find_rsa_key(jwks: &str, kid: &str) -> Result<JWK, jsonwebtoken::errors::Error> {
    // Parse the JWKS
    let jwks: JWKS = serde_json::from_str(jwks)?;

    // Find the RSA key based on the kid
    let rsa_key = jwks
        .keys
        .into_iter()
        .find(|key| key.kid == kid)
        .ok_or(jsonwebtoken::errors::ErrorKind::InvalidIssuer)?;

    Ok(rsa_key)
}

async fn verify_cognito_jwt_token(
    jwt_token: &str,
    user_pool_region: &str,
    user_pool_id: &str,
) -> Result<TokenData<Claims>, jsonwebtoken::errors::Error> {
    // Decode the JWT to get the kid (Key ID)
    let jwt_header = jsonwebtoken::decode_header(jwt_token)?;

    // Fetch JWKS from the well-known URL
    let jwks = fetch_jwks(user_pool_region, user_pool_id).await.unwrap();

    // Find the RSA key in the JWKS based on the kid from the JWT
    let rsa_key = find_rsa_key(&jwks, &jwt_header.kid.unwrap_or_default())?;

    // Create the DecodingKey using the RSA key components
    let decoding_key = DecodingKey::from_rsa_components(&rsa_key.n, &rsa_key.e)?;

    // Set up the validation parameters
    let mut validation = Validation::new(Algorithm::RS256);
    validation.leeway = 5; // Allow a 60-second leeway for token expiration
    validation.validate_exp = true; // Validate expiration claim
    validation.validate_nbf = true; // Validate not-before claim
    validation.set_issuer(&[&format!(
        "https://cognito-idp.{}.amazonaws.com/{}",
        user_pool_region, user_pool_id
    )]);

    // Decode and verify the JWT token
    let token_data = decode::<Claims>(jwt_token, &decoding_key, &validation)?;

    Ok(token_data)
}
