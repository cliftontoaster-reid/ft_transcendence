use dionysus_oidc::{build_rocket, model::config::OidcConfig};
use rocket::{Build, Rocket};
use std::{env, fs, path::PathBuf};

#[derive(Debug, rocket::serde::Deserialize)]
#[serde(crate = "rocket::serde")]
struct ConfigFile {
  oidc: OidcConfig,
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
  build_rocket(config).expect("failed to build OIDC auth service")
}
