//! Hand-rolled HTTP server for provider tests: no extra test dependency,
//! and it exercises the real reqwest path (same pattern as
//! `dionysus_metadata::image_fetch` tests). Records every request line so
//! tests can assert on paths, query params, and auth headers.

use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub(crate) struct MockServer {
  port: u16,
  requests: Arc<Mutex<Vec<String>>>,
}

impl MockServer {
  /// 200 + `Content-Type: application/json`; the same body answers every
  /// connection.
  pub(crate) async fn serve_json(body: String) -> Self {
    Self::serve_sequence(vec![body]).await
  }

  /// Like [`serve_json`](Self::serve_json), but request *n* gets entry *n*
  /// of `bodies` (the last entry repeats) — for flows that hit several
  /// endpoints on one host, e.g. `/find` then `/movie/{id}`.
  pub(crate) async fn serve_sequence(bodies: Vec<String>) -> Self {
    Self::serve("200 OK".to_string(), "application/json".to_string(), bodies).await
  }

  /// Arbitrary status line (404/500 tests); body is still returned.
  pub(crate) async fn serve_status(status: u16, body: &str) -> Self {
    Self::serve(format!("{status} Unknown"), "text/plain".to_string(), vec![body.to_string()]).await
  }

  async fn serve(status_line: String, content_type: String, bodies: Vec<String>) -> Self {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
      .await
      .expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let requests: Arc<Mutex<Vec<String>>> = Arc::default();
    let record = Arc::clone(&requests);
    let bodies = Arc::new(Mutex::new(bodies));
    tokio::spawn(async move {
      loop {
        let Ok((mut socket, _)) = listener.accept().await else {
          break;
        };
        let mut buf = vec![0u8; 16 * 1024];
        let read = socket.read(&mut buf).await.unwrap_or(0);
        let request_index = {
          let mut recorded = record.lock().expect("requests lock");
          recorded.push(String::from_utf8_lossy(&buf[..read]).into_owned());
          recorded.len() - 1
        };
        let body = {
          let bodies = bodies.lock().expect("bodies lock");
          bodies
            .get(request_index)
            .or_else(|| bodies.last())
            .cloned()
            .unwrap_or_default()
        };
        let head = format!(
          "HTTP/1.1 {status_line}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
          body.len()
        );
        let _ = socket.write_all(head.as_bytes()).await;
        let _ = socket.write_all(body.as_bytes()).await;
        let _ = socket.shutdown().await;
      }
    });
    Self { port, requests }
  }

  /// Base URL to hand to a provider's `with_base_url`.
  pub(crate) fn base_url(&self) -> String {
    format!("http://127.0.0.1:{}", self.port)
  }

  /// Raw head of the n-th recorded request (request line + headers).
  pub(crate) fn request(&self, index: usize) -> String {
    self
      .requests
      .lock()
      .expect("requests lock")
      .get(index)
      .cloned()
      .unwrap_or_else(|| format!("<no request #{index}>"))
  }

  pub(crate) fn request_count(&self) -> usize {
    self.requests.lock().expect("requests lock").len()
  }
}
