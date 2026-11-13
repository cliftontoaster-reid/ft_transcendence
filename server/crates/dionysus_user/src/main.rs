mod service;

use dionysus_proto::dionysus::user::v1::user_service_server::UserServiceServer;
use service::UserServiceImpl;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  tracing_subscriber::fmt()
    .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
    .init();

  let addr: std::net::SocketAddr = std::env::var("USER_LISTEN_ADDR")
    .unwrap_or_else(|_| "127.0.0.1:50052".to_string())
    .parse()?;

  // Panics when DATABASE_URL is missing or unreachable: the service is useless without it.
  let connection = dionysus_db::establish_connection();

  tracing::info!(%addr, "starting UserService");

  tonic::transport::Server::builder()
    .add_service(UserServiceServer::new(UserServiceImpl::new(connection)))
    .serve(addr)
    .await?;

  Ok(())
}
