mod handlers;
mod models;
mod routes;

// A thread-safe reference-counting pointer. ‘Arc’ stands for ‘Atomically Reference Counted’.
use std::sync::Arc;

use handlebars::Handlebars;

use redis;
// The web framework we are using. It provides a lot of utilities for building web applications.
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get, Extension,
    Json, Router,
};

use axum_template::engine::Engine;
// For generate random number.
use rand::Rng;

// For serialization and deserialization of data. Most popular Rust library for this.
use serde::{Deserialize, Serialize};

// For error handling. This library provides a convenient derive macro for the standard library’s std::error::Error trait.
use thiserror::Error;

// An event-driven, non-blocking I/O platform for writing asynchronous applications.
use tokio::{fs::File, io::AsyncReadExt, sync::RwLock};

// For working with MySQL database. sqlx support other types of databases as well.
use sqlx::mysql::{MySqlPool, MySqlPoolOptions};

// For loading environment variables from a .env file.
use dotenv::dotenv;

use routes::route::create_router;

// Type alias for our engine. For this example, we are using Handlebars
type AppEngine = Engine<Handlebars<'static>>;

#[derive(Debug, Serialize)]
pub struct Person {
    name: String,
}

#[derive(Default, Clone)]
struct AppState2 {
    numbers: Vec<i32>,
}

#[derive(Clone, Debug)]
struct AppState {
    db: MySqlPool,
    view_engine: AppEngine,
}

// Example to keep states of the app. We can use trait objects for shared state
// Sample for trait object state:
// https://github.com/tokio-rs/axum/blob/8854e660e9ab07404e5bb8e30b92311d3848de05/examples/error-handling-and-dependency-injection/src/main.rs#L124
type AppStateType = Arc<RwLock<AppState2>>;

#[tokio::main]
async fn main() {
    // Load environment variables from .env file.
    dotenv().ok();
    // Set up the Handlebars engine with the same route paths as the Axum router
    let mut hbs = Handlebars::new();
    hbs.register_template_string("/api/:name", "<h1>Hello HandleBars!</h1><p>{{name}}</p>")
        .unwrap();
    
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must set");
    let pool = match MySqlPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await{
        Ok(pool) => {
            println!("✅ Connection to the database is successful!");
            pool
        }
        Err(err) => {
            println!("❌ Failed to connect to the database: {:?}", err);
            std::process::exit(1);
        }
    };
    let pool = Arc::new(AppState { db: pool, view_engine: Engine::from(hbs) });
    
    // Set up the Redis client
    let redis_url = std::env::var("REDIS_URL").expect("DATABASE_URL must set");
    let rdc = redis::Client::open(redis_url).unwrap();
    let app = Router::new()
        .route("/", get(hello_world).post(post_hello_world))
        .route("/healthcheck", get(health_check))
        .route("/greet/:name", get(greet_path))
        .route("/greet", get(greet_query).post(greet_body))
        .route("/lookup/:number", get(look_it_up))
        .route("/random", get(return_something_random))
        .merge(numbers_management())
        .with_state(AppStateType::default())
        //.with_state(pool)
        // Let's add additional routes. Note that we can structure complex
        // routing hierarchies using methods like merge and nest.
        .merge(pingpong())
        .nest("/kingkong", kingkong())
        //.route("/:name", get(get_name))
        .merge(poem().merge(create_router(pool)))
        .layer(Extension(rdc));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn hello_world() -> &'static str {
    "Hello, World1!"
}

async fn post_hello_world() -> &'static str {
    "Hello, World1 post!"
}

// Two functions that return a router. This is very useful in larger applications
// with lots of routes.
fn pingpong() -> Router {
    Router::new().route("/ping", get(|| async { "pong" }))
}

fn kingkong() -> Router {
    async fn king() -> &'static str {
        "Kong"
    }
    Router::new().route("/king", get(king))
}

// Path is an "Extractor". Extractors are used to extract data from the request.
// .route("/greet/:name", get(greet_path))
async fn greet_path(Path(name): Path<String>) -> String {
    format!("Hello, {}!", name)
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct QueryParameters {
    salutation: Option<String>,
    name: Option<String>,
}

// Query is also an "Extractor".
// .route("/greet", get(greet_query))
async fn greet_query(Query(params): Query<QueryParameters>) -> String {
    let salutation = params.salutation.unwrap_or_else(|| "Hello".to_string());
    let name = params.name.unwrap_or_else(|| "World".to_string());
    format!("{}, {}!", salutation, name)
}

// Json is the third commonly used extractor.
// .route("/greet", get(greet_query).post(greet_body))
async fn greet_body(Json(params): Json<QueryParameters>) -> String {
    let salutation = params.salutation.unwrap_or_else(|| "Hello".to_string());
    let name = params.name.unwrap_or_else(|| "World".to_string());
    format!("{}, {}!", salutation, name)
}

// NOTE: You can learn all details about extractor at
// https://docs.rs/axum/latest/axum/extract/index.html#defining-custom-extractors

#[derive(Serialize)]
struct LookupResult {
    number: i32,
    found: bool,
}

// A lot of types implement IntoResponse, including tuples.
// Read more at https://docs.rs/axum/latest/axum/response/trait.IntoResponse.html
// .route("/lookup", get(look_it_up))
async fn look_it_up(Path(number): Path<i32>) -> impl IntoResponse {
    // Let's say that only odd numbers are "found"
    match number % 2 {
        1 => (
            StatusCode::OK,
            Json(LookupResult {
                number,
                found: true,
            }),
        ),
        _ => (
            StatusCode::NOT_FOUND,
            Json(LookupResult {
                number,
                found: false,
            }),
        ),
    }
}

// The easiest way to return different data types from a handler
// is to convert them into Response, which implements IntoRespose.
// .route("/random", get(return_something_random))
async fn return_something_random() -> impl IntoResponse {
    // Generate random number between 0 and 2 (including)
    match rand::thread_rng().gen_range(0..3) {
        0 => "Hello, World!".into_response(),
        1 => StatusCode::NOT_IMPLEMENTED.into_response(),
        _ => Json(42).into_response(),
    }
}

fn numbers_management() -> Router<AppStateType> {
    // State is another extractor. It can be used to extract shared state.
    // Read more at https://docs.rs/axum/latest/axum/index.html#using-the-state-extractor
    // .merge(numbers_management())
    // .with_state(Arc::new(RwLock::new(AppState::default())))
    async fn get_numbers(State(state): State<AppStateType>) -> impl IntoResponse {
        Json(state.read().await.numbers.clone())
    }

    async fn add_number(
        State(state): State<AppStateType>,
        Json(new_number): Json<i32>,
    ) -> impl IntoResponse {
        let mut writable_state = state.write().await;
        writable_state.numbers.push(new_number);
        Json(writable_state.numbers.clone())
    }

    Router::new().route("/numbers", get(get_numbers).post(add_number))
}

fn poem() -> Router {
    // Possible errors that can occur when reading poem from file.
    // Note that this uses thiserror.
    #[derive(Error, Debug)]
    pub enum PoemError {
        #[error("error accessing file")]
        FileAccess(#[from] tokio::io::Error),
        #[error("error parsing yaml")]
        YamlParse(#[from] serde_yaml::Error),
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Poem {
        pub title: String,
        pub text: String,
    }

    // Let's write a helper method that reads a poem from a file.
    async fn read_from_file(path: &str) -> Result<Poem, PoemError> {
        let mut contents = String::new();
        File::open(path)
            .await?
            .read_to_string(&mut contents)
            .await?;
        Ok(serde_yaml::from_str(&contents)?)
    }

    // Implement IntoResponse for our error type.
    impl IntoResponse for PoemError {
        fn into_response(self) -> Response {
            let (status, error_message) = match self {
                PoemError::FileAccess(ioe) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Error while accessing file: {ioe}"),
                ),
                PoemError::YamlParse(ye) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Error in YMAL file: {ye}"),
                ),
            };

            let body = Json(error_message);
            (status, body).into_response()
        }
    }

    // Handler turning our poem into HTML.
    async fn get_poem() -> Result<Html<String>, PoemError> {
        let poem = read_from_file("poem.yaml").await?;
        Ok(Html(format!(
            r#"
            <html>
                <head>
                    <title>{}</title>
                </head>
                <body>
                    <h1>{}</h1>
                    <pre>{}</pre>
                </body>
            </html>
        "#,
            poem.title, poem.title, poem.text
        )))
    }

    Router::new().route("/poem", get(get_poem))
}

async fn health_check(Extension(rdc): Extension<redis::Client>) -> impl IntoResponse {
    let mut redis_conn = rdc.get_connection().expect("failed to connect to Redis");
    let _: () = redis::cmd("SET").arg("healthcheck").arg("OK").query(&mut redis_conn).expect("failed to execute SET for 'foo'");
    
    const MESSAGE: &str = "API Services";
    let json_response = serde_json::json!({
        "status": "ok2",
        "message": MESSAGE,
    });
    (StatusCode::OK, Json(json_response))
}
