use dionysus_oidc::{
  login::{CallbackRequest, LoginUrlError, LoginUrlService},
  model::config::OidcConfig,
};
use rocket::{
  Build, Rocket, State, form::FromForm, http::Status, response::Redirect, routes, serde::json::Json,
};
use std::{env, fs, path::PathBuf};
use url::Url;

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
pub async fn handle_openid_callback(
  query: CallbackQuery,
  service: &State<LoginUrlService>,
) -> Result<Json<dionysus_oidc::login::AuthenticatedUser>, Status> {
  let callback = CallbackRequest {
    code: query.code,
    state: query.state,
    error: query.error,
    error_description: query.error_description,
  };

  service
    .handle_callback(callback)
    .await
    .map(Json)
    .map_err(callback_error_status)
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

  rocket::build()
    .manage(service)
    .mount("/", routes![generate_openid_url, handle_openid_callback])
}
