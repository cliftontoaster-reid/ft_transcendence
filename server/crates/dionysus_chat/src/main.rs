mod handler;
mod state;

use crate::state::AppState;
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
  tracing_subscriber::fmt()
    .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
    .init();

  let addr: SocketAddr = "0.0.0.0:9001".parse().unwrap();
  let listener = TcpListener::bind(addr).await.unwrap();
  tracing::info!("dionysus_chat listening on {}", addr);

  let state = AppState::new(100);

  loop {
    match listener.accept().await {
      Ok((stream, _)) => {
        let state = state.clone();
        tokio::spawn(async move {
          handler::handle_connection(stream, state).await;
        });
      }
      Err(e) => tracing::error!("accept error: {}", e),
    }
  }
}
