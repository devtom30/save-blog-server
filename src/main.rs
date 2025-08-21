use axum::extract::State;
use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router
};
use serde::{Deserialize, Serialize, Serializer};
use std::sync::{Arc, Mutex};
use serde::ser::SerializeStruct;
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    users: Arc<Mutex<Vec<User>>>
}

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    let app_state = AppState {
        users: Arc::new(Mutex::new(vec![]))
    };

    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root))
        // `POST /users` goes to `create_user`
        .route("/users", post(create_user))
        .route("/users", get(get_users))
        .with_state(app_state);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// basic handler that responds with a static string
async fn root() -> &'static str {
    "Hello, World!"
}

async fn create_user(
    // this argument tells axum to parse the request body
    // as JSON into a `CreateUser` type
    State(state): State<AppState>,
    Json(payload): Json<CreateUser>
) -> (StatusCode, Json<User>) {
    // insert your application logic here
    let user = User {
        id: Uuid::new_v4(),
        username: payload.username,
    };

    match state.users.lock() {
        Ok(mut users) => {
            users.push(user.clone());
        }
        _ => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(user))
        }
    }

    // this will be converted into a JSON response
    // with a status code of `201 Created`
    (StatusCode::CREATED, Json(user))
}

async fn get_users(
    State(state): State<AppState>
) -> (StatusCode, Json<Vec<User>>) {
    match state.users.lock() {
        Ok(users) => {
            (StatusCode::OK, Json(users.clone()))
        }
        _ => {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(Vec::new()))
        }
    }
}

// the input to our `create_user` handler
#[derive(Deserialize)]
struct CreateUser {
    username: String,
}

// the output to our `create_user` handler
#[derive(Clone)]
struct User {
    id: Uuid,
    username: String,
}

impl Serialize for User {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        let mut state = serializer.serialize_struct("User", 2)?;
        state.serialize_field("id", &self.id.to_string().as_str())?;
        state.serialize_field("username", &self.username)?;
        state.end()
    }
}