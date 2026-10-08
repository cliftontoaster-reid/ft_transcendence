mod bitrate;
mod cache;
mod config;
mod manifest;
mod params;
mod proto;
mod segments;
mod source;
mod window;

use config::AppConfig;

pub(crate) fn build_rocket(config: AppConfig) -> rocket::Rocket<rocket::Build> {
  rocket::build().manage(config).mount(
    "/",
    rocket::routes![
      segments::manifest_route,
      segments::video_segment,
      segments::audio_segment
    ],
  )
}

#[rocket::launch]
fn rocket() -> _ {
  let _ = tracing_subscriber::fmt()
    .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
    .try_init();
  build_rocket(AppConfig::from_env())
}
