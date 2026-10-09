mod codec;
mod jobs;
mod proto;

use jobs::JobServer;
use proto::dionysus::encoder::v1::job_service_server::JobServiceServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  tracing_subscriber::fmt()
    .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
    .init();

  let max_concurrent: usize = std::env::var("MAX_CONCURRENT")
    .ok()
    .and_then(|v| v.parse().ok())
    .unwrap_or_else(num_cpus_default);

  let addr: std::net::SocketAddr = std::env::var("LISTEN_ADDR")
    .unwrap_or_else(|_| "[::1]:50051".to_string())
    .parse()?;

  let server = JobServer::new(max_concurrent).await;
  tracing::info!(%addr, max_concurrent, "starting JobService");
  println!("Starting JobService on {addr} (max {max_concurrent} concurrent jobs)");

  tonic::transport::Server::builder()
    .add_service(JobServiceServer::new(server))
    .serve(addr)
    .await?;

  Ok(())
}

fn num_cpus_default() -> usize {
  std::thread::available_parallelism()
    .map_or(4, |n| n.get())
    .clamp(1, 8)
}
