use crate::state::{AppState, ConnId};
use dionysus_proto::dionysus::chat::v1::{ClientFrame, ServerFrame};
use dionysus_proto::dionysus::user::v1::User;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;

use tokio_tungstenite::tungstenite::Message as WsMessage;

pub async fn handle_connection(stream: TcpStream, state: AppState) {
  let ws = match tokio_tungstenite::accept_async(stream).await {
    Ok(ws) => ws,
    Err(e) => {
      tracing::warn!("ws upgrade failed: {}", e);
      return;
    }
  };
  let (mut sink, mut stream) = ws.split();
  let (tx, mut rx) = mpsc::channel::<ServerFrame>(64);
  let conn_id = state.register_connection(tx).await;

  // writer task
  let writer = tokio::spawn(async move {
    while let Some(frame) = rx.recv().await {
      let bytes = prost::Message::encode_to_vec(&frame);
      if sink.send(WsMessage::Binary(bytes.into())).await.is_err() {
        break;
      }
    }
  });

  while let Some(msg) = stream.next().await {
    match msg {
      Ok(WsMessage::Binary(data)) => match prost::Message::decode(&data[..]) {
        Ok(ClientFrame {
          payload: Some(payload),
        }) => {
          if let Err(e) = dispatch(&state, conn_id, payload).await {
            let _ = send_error(&state, conn_id, "BAD_REQUEST", &e).await;
          }
        }
        Ok(_) => {
          let _ = send_error(&state, conn_id, "BAD_FRAME", "missing payload").await;
        }
        Err(e) => {
          tracing::warn!("decode error: {}", e);
          let _ = send_error(&state, conn_id, "DECODE_ERROR", &e.to_string()).await;
          break;
        }
      },
      Ok(WsMessage::Close(_)) => break,
      Ok(_) => {}
      Err(e) => {
        tracing::warn!("ws error: {}", e);
        break;
      }
    }
  }

  let affected = state.remove_connection(conn_id).await;
  for channel_id in affected {
    let frame = ServerFrame {
      payload: Some(
        dionysus_proto::dionysus::chat::v1::server_frame::Payload::Left(
          dionysus_proto::dionysus::chat::v1::UserLeft {
            channel_id: Some(dionysus_proto::dionysus::room::v1::RoomId {
              value: Some(dionysus_proto::dionysus::common::v1::Uuid {
                value: channel_id.clone().into(),
              }),
            }),
            user_id: None,
          },
        ),
      ),
    };
    state.broadcast(&channel_id, frame).await;
  }
  writer.abort();
}

async fn dispatch(
  state: &AppState,
  conn_id: ConnId,
  payload: dionysus_proto::dionysus::chat::v1::client_frame::Payload,
) -> Result<(), String> {
  use dionysus_proto::dionysus::chat::v1::client_frame::Payload;
  use dionysus_proto::dionysus::chat::v1::server_frame::Payload as SPayload;
  match payload {
    Payload::Identify(ident) => {
      let user_id = ident
        .user_id
        .and_then(|u| u.value)
        .map(|u| u.value.to_vec())
        .ok_or("missing user_id")?;
      // In MVP we don't validate token; just store user.
      let user = None::<User>;
      state.identify(conn_id, user_id, user).await;
      Ok(())
    }
    Payload::Join(join) => {
      let channel_id = join
        .channel_id
        .and_then(|r| r.value)
        .map(|u| u.value.to_vec())
        .ok_or("missing channel_id")?;
      let room_name = format!("channel-{}", hex(&channel_id));
      let info = state.join(conn_id, channel_id.clone(), room_name).await?;
      // send snapshot to joiner
      let conn_tx = {
        let st = state.inner.read().await;
        st.connections.get(&conn_id).map(|c| c.sender.clone())
      };
      if let Some(tx) = conn_tx {
        let _ = tx
          .send(ServerFrame {
            payload: Some(SPayload::Snapshot(info.snapshot)),
          })
          .await;
      }
      // broadcast joined
      if let Some(user) = info.user {
        let frame = ServerFrame {
          payload: Some(SPayload::Joined(
            dionysus_proto::dionysus::chat::v1::UserJoined {
              channel_id: Some(dionysus_proto::dionysus::room::v1::RoomId {
                value: Some(dionysus_proto::dionysus::common::v1::Uuid {
                  value: channel_id.clone().into(),
                }),
              }),
              user: Some(user),
            },
          )),
        };
        state.broadcast(&channel_id, frame).await;
      }
      Ok(())
    }
    Payload::Leave(leave) => {
      let channel_id = leave
        .channel_id
        .and_then(|r| r.value)
        .map(|u| u.value.to_vec())
        .ok_or("missing channel_id")?;
      let user_id = state.leave(conn_id, &channel_id).await?;
      let frame = ServerFrame {
        payload: Some(SPayload::Left(
          dionysus_proto::dionysus::chat::v1::UserLeft {
            channel_id: Some(dionysus_proto::dionysus::room::v1::RoomId {
              value: Some(dionysus_proto::dionysus::common::v1::Uuid {
                value: channel_id.clone().into(),
              }),
            }),
            user_id: Some(dionysus_proto::dionysus::user::v1::UserId {
              value: Some(dionysus_proto::dionysus::common::v1::Uuid { value: user_id }),
            }),
          },
        )),
      };
      state.broadcast(&channel_id, frame).await;
      Ok(())
    }
    Payload::Send(send) => {
      let room_id = send
        .room_id
        .and_then(|r| r.value)
        .map(|u| u.value.to_vec())
        .ok_or("missing room_id")?;
      let text = send.message.clone();
      let replies_to = send.replies_to.clone();
      let msg = state
        .send_message(conn_id, room_id.clone(), text, replies_to)
        .await?;
      let frame = ServerFrame {
        payload: Some(SPayload::Message(msg)),
      };
      state.broadcast(&room_id, frame).await;
      Ok(())
    }
    Payload::Edit(edit) => {
      let msg_id = edit
        .id
        .and_then(|i| i.value)
        .map(|u| u.value.to_vec())
        .ok_or("missing id")?;
      let (msg, channel_id) = state
        .edit_message(conn_id, msg_id.clone(), edit.new_message.clone())
        .await?;
      let frame = ServerFrame {
        payload: Some(SPayload::Edited(
          dionysus_proto::dionysus::chat::v1::MessageEdited {
            id: Some(dionysus_proto::dionysus::chat::v1::MessageId {
              value: Some(dionysus_proto::dionysus::common::v1::Uuid {
                value: msg_id.into(),
              }),
            }),
            new_message: edit.new_message,
            updated_at: msg.updated_at,
          },
        )),
      };
      // Broadcast to the channel so all subscribers see the edit
      state.broadcast(&channel_id, frame).await;
      Ok(())
    }
    Payload::Ping(_) => {
      let conn_tx = {
        let st = state.inner.read().await;
        st.connections.get(&conn_id).map(|c| c.sender.clone())
      };
      if let Some(tx) = conn_tx {
        let _ = tx
          .send(ServerFrame {
            payload: Some(SPayload::Pong(dionysus_proto::dionysus::chat::v1::Pong {})),
          })
          .await;
      }
      Ok(())
    }
  }
}

async fn send_error(state: &AppState, conn_id: ConnId, code: &str, message: &str) {
  use dionysus_proto::dionysus::chat::v1::server_frame::Payload as SPayload;
  let conn_tx = {
    let st = state.inner.read().await;
    st.connections.get(&conn_id).map(|c| c.sender.clone())
  };
  if let Some(tx) = conn_tx {
    let _ = tx
      .send(ServerFrame {
        payload: Some(SPayload::Error(dionysus_proto::dionysus::chat::v1::Error {
          code: code.to_string(),
          message: message.to_string(),
        })),
      })
      .await;
  }
}

fn hex(bytes: &[u8]) -> String {
  bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
