use dionysus_proto::dionysus::chat::v1::{Message, ServerFrame};
use dionysus_proto::dionysus::common::v1::Uuid;
use dionysus_proto::dionysus::room::v1::Room;
use dionysus_proto::dionysus::user::v1::User;
use dionysus_proto::google::r#type::DateTime;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{RwLock, mpsc};

pub type ConnId = u64;

#[derive(Clone)]
pub struct AppState {
  pub inner: Arc<RwLock<InnerState>>,
  pub history_limit: usize,
}

pub struct InnerState {
  pub channels: HashMap<Vec<u8>, Channel>,
  pub connections: HashMap<ConnId, Connection>,
  pub next_conn: ConnId,
}

pub struct Channel {
  pub room: Room,
  pub members: HashSet<Vec<u8>>,
  pub subscribers: HashMap<ConnId, mpsc::Sender<ServerFrame>>,
  pub history: VecDeque<Message>,
}

pub struct Connection {
  pub user_id: Option<Vec<u8>>,
  pub user: Option<User>,
  pub channels: HashSet<Vec<u8>>,
  pub sender: mpsc::Sender<ServerFrame>,
}

impl AppState {
  pub fn new(history_limit: usize) -> Self {
    Self {
      inner: Arc::new(RwLock::new(InnerState {
        channels: HashMap::new(),
        connections: HashMap::new(),
        next_conn: 1,
      })),
      history_limit,
    }
  }

  pub async fn register_connection(&self, sender: mpsc::Sender<ServerFrame>) -> ConnId {
    let mut state = self.inner.write().await;
    let id = state.next_conn;
    state.next_conn += 1;
    state.connections.insert(
      id,
      Connection {
        user_id: None,
        user: None,
        channels: HashSet::new(),
        sender,
      },
    );
    id
  }

  pub async fn remove_connection(&self, conn_id: ConnId) -> Vec<Vec<u8>> {
    let mut state = self.inner.write().await;
    let mut affected = Vec::new();
    if let Some(conn) = state.connections.remove(&conn_id) {
      for channel_id in &conn.channels {
        if let Some(channel) = state.channels.get_mut(channel_id) {
          channel.subscribers.remove(&conn_id);
          if let Some(uid) = &conn.user_id {
            channel.members.remove(uid);
          }
          affected.push(channel_id.clone());
        }
      }
    }
    affected
  }

  pub async fn identify(&self, conn_id: ConnId, user_id: Vec<u8>, user: Option<User>) {
    let mut state = self.inner.write().await;
    if let Some(conn) = state.connections.get_mut(&conn_id) {
      conn.user_id = Some(user_id);
      conn.user = user;
    }
  }

  pub async fn join(
    &self,
    conn_id: ConnId,
    channel_id: Vec<u8>,
    room_name: String,
  ) -> Result<JoinResult, String> {
    let mut state = self.inner.write().await;
    let (user_id, user) = {
      let conn = state
        .connections
        .get(&conn_id)
        .ok_or("unknown connection")?;
      (
        conn.user_id.clone().ok_or("not identified")?,
        conn.user.clone(),
      )
    };
    let sender = state
      .connections
      .get(&conn_id)
      .map(|c| c.sender.clone())
      .ok_or("unknown connection")?;

    let channel = state
      .channels
      .entry(channel_id.clone())
      .or_insert_with(|| Channel {
        room: Room {
          id: Some(dionysus_proto::dionysus::room::v1::RoomId {
            value: Some(Uuid {
              value: channel_id.clone().into(),
            }),
          }),
          name: room_name,
          participant: Vec::new(),
          age: 0,
        },
        members: HashSet::new(),
        subscribers: HashMap::new(),
        history: VecDeque::new(),
      });

    channel.members.insert(user_id.clone());
    if let Some(user) = user.clone() {
      let already = channel.room.participant.iter().any(|p| {
        p.id
          .as_ref()
          .and_then(|i| i.value.as_ref())
          .map(|u| u.value.as_slice())
          == Some(user_id.as_slice())
      });
      if !already {
        channel.room.participant.push(user);
      }
    }

    channel.subscribers.insert(conn_id, sender);

    let snapshot = dionysus_proto::dionysus::chat::v1::ChannelSnapshot {
      room: Some(channel.room.clone()),
      recent_messages: channel.history.iter().cloned().collect(),
    };

    if let Some(conn) = state.connections.get_mut(&conn_id) {
      conn.channels.insert(channel_id.clone());
    }
    let joined_user = state.connections.get(&conn_id).and_then(|c| c.user.clone());

    Ok(JoinResult {
      snapshot,
      user: joined_user,
    })
  }

  pub async fn leave(&self, conn_id: ConnId, channel_id: &[u8]) -> Result<Vec<u8>, String> {
    let mut state = self.inner.write().await;
    let user_id = state
      .connections
      .get(&conn_id)
      .and_then(|c| c.user_id.clone())
      .ok_or_else(|| "not identified".to_string())?;
    if let Some(channel) = state.channels.get_mut(channel_id) {
      channel.subscribers.remove(&conn_id);
      channel.members.remove(&user_id);
      if let Some(conn) = state.connections.get_mut(&conn_id) {
        conn.channels.remove(channel_id);
      }
    }
    Ok(user_id)
  }

  pub async fn send_message(
    &self,
    conn_id: ConnId,
    room_id: Vec<u8>,
    text: String,
    replies_to: Option<dionysus_proto::dionysus::chat::v1::MessageId>,
  ) -> Result<Message, String> {
    let mut state = self.inner.write().await;
    let user_id = {
      let conn = state
        .connections
        .get(&conn_id)
        .ok_or("unknown connection")?;
      conn.user_id.clone().ok_or("not identified")?
    };
    let channel = state.channels.get_mut(&room_id).ok_or("unknown channel")?;
    if !channel.subscribers.contains_key(&conn_id) {
      return Err("not in channel".to_string());
    }
    let id_bytes = uuid::Uuid::new_v4().as_bytes().to_vec();
    let now = current_timestamp();
    let msg = Message {
      id: Some(dionysus_proto::dionysus::chat::v1::MessageId {
        value: Some(Uuid {
          value: id_bytes.into(),
        }),
      }),
      sender_id: Some(dionysus_proto::dionysus::user::v1::UserId {
        value: Some(Uuid {
          value: user_id.clone().into(),
        }),
      }),
      room_id: Some(dionysus_proto::dionysus::room::v1::RoomId {
        value: Some(Uuid {
          value: room_id.clone().into(),
        }),
      }),
      message: text,
      replies_to,
      created_at: Some(now.clone()),
      updated_at: Some(now),
    };
    channel.history.push_back(msg.clone());
    while channel.history.len() > self.history_limit {
      channel.history.pop_front();
    }
    Ok(msg)
  }

  pub async fn edit_message(
    &self,
    conn_id: ConnId,
    msg_id: Vec<u8>,
    new_text: String,
  ) -> Result<(Message, Vec<u8>), String> {
    let mut state = self.inner.write().await;
    let user_id = state
      .connections
      .get(&conn_id)
      .and_then(|c| c.user_id.clone())
      .ok_or_else(|| "not identified".to_string())?;

    for channel in state.channels.values_mut() {
      if let Some(pos) = channel.history.iter().position(|m| {
        m.id
          .as_ref()
          .and_then(|i| i.value.as_ref())
          .map(|u| u.value.as_slice())
          == Some(msg_id.as_slice())
      }) {
        let m = &mut channel.history[pos];
        let sender = m
          .sender_id
          .as_ref()
          .and_then(|s| s.value.as_ref())
          .map(|u| u.value.as_slice());
        if sender != Some(user_id.as_slice()) {
          return Err("not owner".to_string());
        }
        m.message = new_text;
        let now = current_timestamp();
        m.updated_at = Some(now);
        // Return the message and the channel_id for broadcasting
        let channel_id = channel.room.id.as_ref().and_then(|i| i.value.as_ref()).map(|u| u.value.clone()).unwrap_or_default();
        return Ok((m.clone(), channel_id));
      }
    }
    Err("message not found".to_string())
  }

  pub async fn subscribers(&self, channel_id: &[u8]) -> Vec<mpsc::Sender<ServerFrame>> {
    let state = self.inner.read().await;
    state
      .channels
      .get(channel_id)
      .map(|c| c.subscribers.values().cloned().collect())
      .unwrap_or_default()
  }

  pub async fn broadcast(&self, channel_id: &[u8], frame: ServerFrame) {
    let subs = self.subscribers(channel_id).await;
    for tx in subs {
      let _ = tx.try_send(frame.clone());
    }
  }
}

pub struct JoinResult {
  pub snapshot: dionysus_proto::dionysus::chat::v1::ChannelSnapshot,
  pub user: Option<User>,
}

fn current_timestamp() -> DateTime {
  let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
  let secs = now.as_secs() as i64;
  let nanos = now.subsec_nanos();
  
  // Simple conversion from unix timestamp to date components
  // Using a basic algorithm (not handling leap seconds perfectly, but good enough)
  let days = secs / 86400;
  let mut year = 1970;
  let mut remaining_days = days;
  
  // Approximate year calculation
  while remaining_days >= 365 {
    let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    let days_in_year = if is_leap { 366 } else { 365 };
    if remaining_days >= days_in_year {
      remaining_days -= days_in_year;
      year += 1;
    } else {
      break;
    }
  }
  
  // Month/day calculation (simplified)
  let month_days = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
  let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
  let mut month = 1;
  let mut day = 1;
  let mut rem = remaining_days;
  
  for (i, &md) in month_days.iter().enumerate() {
    let dim = if i == 1 && is_leap { 29 } else { md };
    if rem >= dim {
      rem -= dim;
      month += 1;
    } else {
      day = rem + 1;
      break;
    }
  }
  
  let seconds_of_day = secs % 86400;
  let hours = (seconds_of_day / 3600) as i32;
  let minutes = ((seconds_of_day % 3600) / 60) as i32;
  let seconds = (seconds_of_day % 60) as i32;
  
  DateTime {
    year,
    month,
    day: day as i32,
    hours,
    minutes,
    seconds,
    nanos: nanos as i32,
    time_offset: None,
  }
}
