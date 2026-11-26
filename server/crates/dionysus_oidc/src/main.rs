use argon2::{
  Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
  password_hash::{SaltString, rand_core::OsRng},
};
use chrono::{Duration, Utc};
use diesel::Connection;
use diesel::{OptionalExtension, PgConnection, QueryableByName, RunQueryDsl, sql_query};
use dionysus_db::{establish_connection, run_migrations};
use dionysus_oidc::{
  login::{CallbackRequest, LoginUrlError, LoginUrlService},
  model::config::OidcConfig,
};
use rocket::serde::Deserialize;
use rocket::{
  Build, Rocket, State,
  fairing::{Fairing, Info, Kind},
  form::FromForm,
  http::{Cookie, CookieJar, Header, Method, SameSite, Status},
  response::Redirect,
  routes,
  serde::json::Json,
};
use serde::Serialize;
use std::{env, fs, path::PathBuf, sync::Mutex};
use url::Url;
use uuid::Uuid;

const SESSION_COOKIE: &str = "dionysus_session";
const SESSION_DAYS: i64 = 30;

struct Cors;

#[rocket::async_trait]
impl Fairing for Cors {
  fn info(&self) -> Info {
    Info {
      name: "Authentication CORS",
      kind: Kind::Response,
    }
  }

  async fn on_response<'r>(
    &self,
    request: &'r rocket::Request<'_>,
    response: &mut rocket::Response<'r>,
  ) {
    if matches!(request.method(), Method::Options) {
      response.set_status(Status::Ok);
    }
    if let Some(origin) = request.headers().get_one("Origin")
      && (origin.starts_with("http://localhost:")
        || origin.starts_with("http://127.0.0.1:")
        || origin.starts_with("https://localhost:")
        || origin.starts_with("https://127.0.0.1:"))
    {
      response.set_header(Header::new("Access-Control-Allow-Origin", origin));
    }
    response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
    response.set_header(Header::new("Access-Control-Allow-Headers", "Content-Type"));
    response.set_header(Header::new(
      "Access-Control-Allow-Methods",
      "GET, POST, OPTIONS",
    ));
  }
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct PasswordCredentials {
  email: String,
  password: String,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct RegisterRequest {
  email: String,
  password: String,
  username: String,
}

#[derive(Debug, Serialize, QueryableByName)]
#[serde(crate = "rocket::serde")]
struct AuthUser {
  #[diesel(sql_type = diesel::sql_types::Uuid)]
  id: Uuid,
  #[diesel(sql_type = diesel::sql_types::Text)]
  email: String,
  #[diesel(sql_type = diesel::sql_types::Text)]
  username: String,
}

#[derive(QueryableByName)]
struct OidcUserId {
  #[diesel(sql_type = diesel::sql_types::Uuid)]
  id: Uuid,
}

#[derive(Serialize, QueryableByName)]
#[serde(crate = "rocket::serde")]
struct AuthSession {
  #[diesel(sql_type = diesel::sql_types::Uuid)]
  #[serde(rename = "userId")]
  user_id: Uuid,
}

struct AuthState {
  connection: Mutex<PgConnection>,
}

fn auth_error(message: &str) -> Json<serde_json::Value> {
  Json(serde_json::json!({ "message": message }))
}

fn set_session(
  cookies: &CookieJar<'_>,
  connection: &mut PgConnection,
  user_id: Uuid,
) -> Result<(), Status> {
  let token = Uuid::new_v4();
  let expires_at = Utc::now() + Duration::days(SESSION_DAYS);
  sql_query("INSERT INTO sessions (user_id, jti, expires_at) VALUES ($1, $2, $3)")
    .bind::<diesel::sql_types::Uuid, _>(user_id)
    .bind::<diesel::sql_types::Uuid, _>(token)
    .bind::<diesel::sql_types::Timestamptz, _>(expires_at)
    .execute(connection)
    .map_err(|error| {
      tracing::error!(?error, "could not create auth session");
      Status::InternalServerError
    })?;

  cookies.add(
    Cookie::build((SESSION_COOKIE, token.to_string()))
      .path("/")
      .http_only(true)
      .same_site(SameSite::Lax)
      .max_age(rocket::time::Duration::days(SESSION_DAYS))
      .build(),
  );
  Ok(())
}

fn find_or_create_oidc_user(
  connection: &mut PgConnection,
  authenticated_user: &dionysus_oidc::login::AuthenticatedUser,
) -> Result<Uuid, diesel::result::Error> {
  let find_user = |connection: &mut PgConnection| {
    sql_query(
      "SELECT users.id FROM oidc_identities
       JOIN users ON users.id = oidc_identities.user_id
       WHERE oidc_identities.issuer = $1 AND oidc_identities.subject = $2
         AND users.disabled = false",
    )
    .bind::<diesel::sql_types::Text, _>(&authenticated_user.issuer)
    .bind::<diesel::sql_types::Text, _>(&authenticated_user.subject)
    .get_result::<OidcUserId>(connection)
    .optional()
  };

  if let Some(user) = find_user(connection)? {
    return Ok(user.id);
  }

  let display_name = authenticated_user
    .preferred_username
    .as_deref()
    .map(str::trim)
    .filter(|name| !name.is_empty())
    .unwrap_or("Dionysus user")
    .chars()
    .take(256)
    .collect::<String>();

  let result = connection.transaction::<Uuid, diesel::result::Error, _>(|connection| {
    if let Some(user) = find_user(connection)? {
      return Ok(user.id);
    }

    let user = sql_query("INSERT INTO users (display_name, email) VALUES ($1, $2) RETURNING id")
      .bind::<diesel::sql_types::Text, _>(&display_name)
      .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(
        authenticated_user.email.as_deref(),
      )
      .get_result::<OidcUserId>(connection)?;

    sql_query(
      "INSERT INTO oidc_identities (user_id, issuer, subject, email)
       VALUES ($1, $2, $3, $4)",
    )
    .bind::<diesel::sql_types::Uuid, _>(user.id)
    .bind::<diesel::sql_types::Text, _>(&authenticated_user.issuer)
    .bind::<diesel::sql_types::Text, _>(&authenticated_user.subject)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(
      authenticated_user.email.as_deref(),
    )
    .execute(connection)?;

    Ok(user.id)
  });

  match result {
    Ok(user_id) => Ok(user_id),
    Err(
      error @ diesel::result::Error::DatabaseError(
        diesel::result::DatabaseErrorKind::UniqueViolation,
        _,
      ),
    ) => find_user(connection)?.map(|user| user.id).ok_or(error),
    Err(error) => Err(error),
  }
}

#[rocket::post("/auth/register", data = "<request>")]
fn register(
  request: Json<RegisterRequest>,
  cookies: &CookieJar<'_>,
  state: &State<AuthState>,
) -> Result<Json<AuthUser>, (Status, Json<serde_json::Value>)> {
  let request = request.into_inner();
  let email = request.email.trim().to_lowercase();
  let username = request.username.trim().to_owned();
  if !email.contains('@') || request.password.len() < 8 || !(3..=64).contains(&username.len()) {
    return Err((
      Status::BadRequest,
      auth_error("email, username or password is invalid"),
    ));
  }

  let salt = SaltString::generate(&mut OsRng);
  let password_hash = Argon2::default()
    .hash_password(request.password.as_bytes(), &salt)
    .map_err(|_| {
      (
        Status::InternalServerError,
        auth_error("could not secure password"),
      )
    })?
    .to_string();
  let mut connection = state.connection.lock().map_err(|_| {
    (
      Status::InternalServerError,
      auth_error("database connection unavailable"),
    )
  })?;
  let user = sql_query(
    "INSERT INTO users (display_name, email, username, password_hash)
     VALUES ($1, $2, $3, $4)
     RETURNING id, email, username",
  )
  .bind::<diesel::sql_types::Text, _>(&username)
  .bind::<diesel::sql_types::Text, _>(&email)
  .bind::<diesel::sql_types::Text, _>(&username)
  .bind::<diesel::sql_types::Text, _>(&password_hash)
  .get_result::<AuthUser>(&mut *connection)
  .map_err(|error| {
    tracing::warn!(?error, "could not create user");
    (
      Status::Conflict,
      auth_error("email or username is already in use"),
    )
  })?;

  set_session(cookies, &mut connection, user.id)
    .map_err(|status| (status, auth_error("could not create session")))?;
  Ok(Json(user))
}

#[rocket::post("/auth/login", data = "<request>")]
fn password_login(
  request: Json<PasswordCredentials>,
  cookies: &CookieJar<'_>,
  state: &State<AuthState>,
) -> Result<Json<AuthUser>, (Status, Json<serde_json::Value>)> {
  #[derive(QueryableByName)]
  struct LoginRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Text)]
    email: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    username: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    password_hash: String,
  }

  let email = request.email.trim().to_lowercase();
  let mut connection = state.connection.lock().map_err(|_| {
    (
      Status::InternalServerError,
      auth_error("database connection unavailable"),
    )
  })?;
  let user = sql_query(
    "SELECT id, email, username, password_hash FROM users
     WHERE lower(email) = $1 AND disabled = false",
  )
  .bind::<diesel::sql_types::Text, _>(&email)
  .get_result::<LoginRow>(&mut *connection)
  .optional()
  .map_err(|_| (Status::InternalServerError, auth_error("database error")))?
  .ok_or((Status::Unauthorized, auth_error("invalid credentials")))?;

  let hash = PasswordHash::new(&user.password_hash).map_err(|_| {
    (
      Status::InternalServerError,
      auth_error("invalid password configuration"),
    )
  })?;
  Argon2::default()
    .verify_password(request.password.as_bytes(), &hash)
    .map_err(|_| (Status::Unauthorized, auth_error("invalid credentials")))?;
  set_session(cookies, &mut connection, user.id)
    .map_err(|status| (status, auth_error("could not create session")))?;
  Ok(Json(AuthUser {
    id: user.id,
    email: user.email,
    username: user.username,
  }))
}

#[rocket::post("/auth/logout")]
fn logout(cookies: &CookieJar<'_>, state: &State<AuthState>) -> Status {
  if let Some(cookie) = cookies.get(SESSION_COOKIE) {
    if let (Ok(token), Ok(mut connection)) =
      (Uuid::parse_str(cookie.value()), state.connection.lock())
    {
      let _ = sql_query("DELETE FROM sessions WHERE jti = $1")
        .bind::<diesel::sql_types::Uuid, _>(token)
        .execute(&mut *connection);
    }
  }
  cookies.remove(Cookie::build(SESSION_COOKIE).path("/").build());
  Status::NoContent
}

#[rocket::get("/auth/session")]
fn current_session(
  cookies: &CookieJar<'_>,
  state: &State<AuthState>,
) -> Result<Json<AuthSession>, Status> {
  let token = cookies
    .get(SESSION_COOKIE)
    .and_then(|cookie| Uuid::parse_str(cookie.value()).ok())
    .ok_or(Status::Unauthorized)?;
  let mut connection = state
    .connection
    .lock()
    .map_err(|_| Status::InternalServerError)?;

  sql_query(
    "SELECT users.id AS user_id FROM sessions
     JOIN users ON users.id = sessions.user_id
     WHERE sessions.jti = $1 AND sessions.expires_at > NOW()
       AND sessions.revoked_at IS NULL AND users.disabled = false",
  )
  .bind::<diesel::sql_types::Uuid, _>(token)
  .get_result::<AuthSession>(&mut *connection)
  .map(Json)
  .map_err(|error| {
    if error == diesel::result::Error::NotFound {
      Status::Unauthorized
    } else {
      tracing::error!(?error, "could not validate auth session");
      Status::InternalServerError
    }
  })
}

#[derive(Debug, rocket::serde::Deserialize)]
#[serde(crate = "rocket::serde")]
struct ConfigFile {
  oidc: OidcConfig,
}

#[derive(Debug, FromForm)]
pub struct LoginQuery {
  pub redirect_uri: Option<String>,
}

#[derive(Debug, FromForm)]
pub struct CallbackQuery {
  pub code: Option<String>,
  pub state: Option<String>,
  pub error: Option<String>,
  pub error_description: Option<String>,
}

#[rocket::get("/auth/login/<provider>?<query..>")]
pub async fn generate_openid_url(
  provider: &str,
  query: LoginQuery,
  service: &State<LoginUrlService>,
) -> Result<Redirect, Status> {
  let requested_redirect_uri = query
    .redirect_uri
    .as_deref()
    .map(Url::parse)
    .transpose()
    .map_err(|_| Status::BadRequest)?;

  let request = service
    .authorization_url(provider, requested_redirect_uri.as_ref())
    .await
    .map_err(login_error_status)?;

  Ok(Redirect::to(request.authorization_url.to_string()))
}

#[rocket::get("/auth/callback?<query..>")]
async fn handle_openid_callback(
  query: CallbackQuery,
  cookies: &CookieJar<'_>,
  service: &State<LoginUrlService>,
  state: &State<AuthState>,
) -> Result<Redirect, Status> {
  let callback = CallbackRequest {
    code: query.code,
    state: query.state,
    error: query.error,
    error_description: query.error_description,
  };

  let authenticated_user = service
    .handle_callback(callback)
    .await
    .map_err(callback_error_status)?;

  let mut connection = state
    .connection
    .lock()
    .map_err(|_| Status::InternalServerError)?;
  let user_id = find_or_create_oidc_user(&mut connection, &authenticated_user).map_err(|error| {
    tracing::error!(?error, provider = %authenticated_user.provider, "could not resolve OIDC user");
    Status::InternalServerError
  })?;
  set_session(cookies, &mut connection, user_id)?;

  Ok(Redirect::to(
    env::var("PUBLIC_APP_URL").unwrap_or_else(|_| "http://localhost:5173/".to_string()),
  ))
}

fn login_error_status(error: LoginUrlError) -> Status {
  match error {
    LoginUrlError::UnknownProvider(_)
    | LoginUrlError::MissingRedirectUri(_)
    | LoginUrlError::AmbiguousRedirectUri(_)
    | LoginUrlError::MissingOpenidScope(_)
    | LoginUrlError::InvalidRedirectUri { .. } => Status::BadRequest,
    LoginUrlError::Secret(_)
    | LoginUrlError::InvalidIssuer(_)
    | LoginUrlError::Discovery(_)
    | LoginUrlError::Redis(_)
    | LoginUrlError::Serialization(_)
    | LoginUrlError::MissingCallbackState
    | LoginUrlError::MissingAuthorizationCode
    | LoginUrlError::Provider(_)
    | LoginUrlError::MissingLoginTransaction
    | LoginUrlError::StateMismatch
    | LoginUrlError::TokenExchange(_)
    | LoginUrlError::MissingIdToken
    | LoginUrlError::IdTokenVerification(_) => Status::InternalServerError,
  }
}

fn callback_error_status(error: LoginUrlError) -> Status {
  match error {
    LoginUrlError::MissingCallbackState
    | LoginUrlError::MissingAuthorizationCode
    | LoginUrlError::Provider(_)
    | LoginUrlError::MissingLoginTransaction
    | LoginUrlError::StateMismatch
    | LoginUrlError::MissingIdToken
    | LoginUrlError::IdTokenVerification(_) => Status::BadRequest,
    LoginUrlError::UnknownProvider(_)
    | LoginUrlError::InvalidIssuer(_)
    | LoginUrlError::Secret(_)
    | LoginUrlError::Discovery(_)
    | LoginUrlError::Redis(_)
    | LoginUrlError::Serialization(_)
    | LoginUrlError::TokenExchange(_) => Status::InternalServerError,
    LoginUrlError::MissingRedirectUri(_)
    | LoginUrlError::AmbiguousRedirectUri(_)
    | LoginUrlError::MissingOpenidScope(_)
    | LoginUrlError::InvalidRedirectUri { .. } => Status::BadRequest,
  }
}

fn load_config() -> Result<OidcConfig, Box<dyn std::error::Error>> {
  let path = env::var_os("OIDC_CONFIG")
    .map(PathBuf::from)
    .unwrap_or_else(|| PathBuf::from("config/oidc.toml"));
  let contents = fs::read_to_string(path)?;
  Ok(toml::from_str::<ConfigFile>(&contents)?.oidc)
}

#[rocket::launch]
async fn rocket() -> Rocket<Build> {
  let config = load_config().expect("failed to load OIDC_CONFIG");
  let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/".to_string());
  let redis = redis::Client::open(redis_url).expect("invalid REDIS_URL");
  let service = LoginUrlService::new(config, redis);
  let mut connection = establish_connection();
  run_migrations(&mut connection).expect("failed to run database migrations");

  rocket::build()
    .attach(Cors)
    .manage(service)
    .manage(AuthState {
      connection: Mutex::new(connection),
    })
    .mount(
      "/",
      routes![
        generate_openid_url,
        handle_openid_callback,
        register,
        password_login,
        logout,
        current_session
      ],
    )
}
