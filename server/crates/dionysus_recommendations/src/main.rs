use dionysus_proto::dionysus::recommendations::v1::recommendation_service_server::RecommendationServiceServer;
use dionysus_recommendations::service::RecommendationServiceImpl;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  tracing_subscriber::fmt()
    .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
    .init();

  let addr: std::net::SocketAddr = std::env::var("RECOMMENDATION_LISTEN_ADDR")
    .unwrap_or_else(|_| "127.0.0.1:50053".to_string())
    .parse()?;

  let connection = dionysus_db::establish_connection();

  tracing::info!(%addr, "starting RecommendationService");

  tonic::transport::Server::builder()
    .add_service(RecommendationServiceServer::new(
      RecommendationServiceImpl::new(connection),
    ))
    .serve(addr)
    .await?;

  Ok(())
}
