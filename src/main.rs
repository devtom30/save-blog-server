use axum::extract::State;
use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router
};
use chrono::{DateTime, Local};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Clone)]
struct Save {
    start_time: DateTime<Local>,
    end_time: Option<DateTime<Local>>,
    path: String
}

impl Serialize for Save {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        let mut state = serializer.serialize_struct("User", 3)?;
        state.serialize_field("start_time", &self.start_time.format("%d/%m/%Y %H:%M:%s").to_string())?;
        state.serialize_field("end_time", &self.end_time.map_or("".to_string(), 
                                                         |time| time.format("%d/%m/%Y %H:%M:%s").to_string()))?;
        state.serialize_field("path",  &self.path)?;
        state.end()
    }
}

#[derive(Clone)]
struct AppState {
    users: Arc<Mutex<Vec<User>>>,
    current_save: Arc<Mutex<Option<Save>>>,
    saves_path: String
}

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    let app_state = AppState {
        users: Arc::new(Mutex::new(vec![])),
        current_save: Arc::new(Mutex::new(None)),
        saves_path: String::from("./saves/")
    };

    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root))
        // `POST /users` goes to `create_user`
        .route("/users", post(create_user))
        .route("/users", get(get_users))
        .route("/parse", post(parse_html))
        .route("/attach", post(attach_file))
        .route("/save/init", post(init_save))
        .route("/save/end", post(end_save))
        .route("/save", get(get_current_save))
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

async fn parse_html() -> StatusCode {
    StatusCode::OK
}

async fn attach_file() -> StatusCode {
    StatusCode::OK
}

async fn init_save(
    State(state): State<AppState>
) -> StatusCode {
    match state.current_save.lock() {
        Ok(mut save_option) => {
            let save_option_cloned = save_option.clone();
            if let Some(save) = save_option_cloned {
                return StatusCode::NOT_ACCEPTABLE
            } else {
                *save_option = Some(Save {
                    start_time: DateTime::from(Local::now()),
                    end_time: None,
                    path: "".to_string(),
                })
            }
            
        }
        Err(_) => {
            return StatusCode::INTERNAL_SERVER_ERROR
        }
    }
    
    StatusCode::OK
}

async fn end_save() -> StatusCode {
    StatusCode::OK
}

async fn get_current_save(
    State(state): State<AppState>
) -> (StatusCode, Json<Option<Save>>) 
{
    match state.current_save.lock() {
        Ok(save_option) => {
            match save_option.clone() {
                None => {
                    (StatusCode::NOT_FOUND, Json(None))
                }
                Some(save) => {
                    (StatusCode::OK, Json(Some(Save{
                        start_time: save.start_time.clone(),
                        end_time: save.end_time.clone(),
                        path: save.path.clone(),
                    })))
                }
            }
        }
        _ => { 
            (StatusCode::INTERNAL_SERVER_ERROR, Json(None)) 
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