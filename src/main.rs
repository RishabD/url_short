use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Json, Router,
};
use dotenv::dotenv;
use serde::Deserialize;
use sled;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;

struct AppState {
    kv: sled::Db,
}

#[tokio::main]
async fn main() {
    let server_addr = get_env_vars();
    let server_state = Arc::new(AppState {
        kv: sled::open("sled_key_to_url").unwrap(),
    });
    let app = Router::new()
        .route("/get_url", get(get_url))
        .route("/set_url", post(set_url))
        .route("/list_urls", get(list_urls))
        .with_state(server_state);
    let listener = tokio::net::TcpListener::bind(server_addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn get_env_vars() -> String {
    dotenv().unwrap();
    return env::var("SERVER_ADDR").unwrap();
}

async fn get_url(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let Some(key) = params.get("key") else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let Some(redirect_url) = get_url_using_key(key, state) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    return Redirect::to(&redirect_url).into_response();
}

#[derive(Deserialize)]
struct SetUrlRequestBody {
    key: String,
    url: Option<String>,
}

async fn set_url(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SetUrlRequestBody>,
) -> impl IntoResponse {
    if set_url_using_key(&payload.key, &payload.url, state) {
        return StatusCode::OK.into_response();
    }

    return (
        StatusCode::BAD_REQUEST,
        "Key is already taken or there was an issue getting the url.",
    )
        .into_response();
}

async fn list_urls(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let Some(key_to_url_map) = get_key_to_url_map(state) else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    return Json(key_to_url_map).into_response();
}

fn get_url_using_key(key: &String, state: Arc<AppState>) -> Option<String> {
    let Ok(Some(url_bytes)) = state.kv.get(key) else {
        return None;
    };
    return Some(String::from_utf8(url_bytes.to_vec()).unwrap());
}

fn set_url_using_key(key: &String, url_or_none: &Option<String>, state: Arc<AppState>) -> bool {
    if let Some(url) = url_or_none {
        return state.kv.insert(key.clone(), url.as_bytes()).is_ok();
    }
    return state.kv.remove(key).is_ok();
}

fn get_key_to_url_map(state: Arc<AppState>) -> Option<HashMap<String, String>> {
    let mut key_to_url_map: HashMap<String, String> = HashMap::new();
    for result in state.kv.iter() {
        let Ok((key_bytes, url_bytes)) = result else {
            return None;
        };
        let key = String::from_utf8(key_bytes.to_vec()).unwrap();
        let url = String::from_utf8(url_bytes.to_vec()).unwrap();
        key_to_url_map.insert(key, url);
    }
    return Some(key_to_url_map);
}
