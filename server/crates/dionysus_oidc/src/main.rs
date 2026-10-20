use dionysus_oidc::{
  login::{LoginUrlError, LoginUrlService},
  model::config::OidcConfig,
};
use rocket::{Build, Rocket, State, form::FromForm, http::Status, response::Redirect, routes};
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
    | LoginUrlError::Serialization(_) => Status::InternalServerError,
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
    .mount("/", routes![generate_openid_url])
}
