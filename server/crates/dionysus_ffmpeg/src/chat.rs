use std::{
  collections::HashMap,
  sync::{Arc, Mutex},
  time::{SystemTime, UNIX_EPOCH},
};

use tonic::{Request, Response, Status};

use crate::proto::dionysus::chat::v1::{
  ChatMessage, GetConversationRequest, GetConversationResponse, SendMessageRequest,
  SendMessageResponse, UserId, chat_service_server::ChatService,
};

pub struct ChatServiceImpl {
  messages: Arc<Mutex<HashMap<String, Vec<ChatMessage>>>>,
}

impl ChatServiceImpl {
  pub fn new() -> Self {
    Self {
      messages: Arc::new(Mutex::new(HashMap::new())),
    }
  }

  fn conversation_key(user_a: &str, user_b: &str) -> String {
    if user_a <= user_b {
      format!("{user_a}:{user_b}")
    } else {
      format!("{user_b}:{user_a}")
    }
  }

  fn parse_user_id(value: Option<UserId>, field: &str) -> Result<UserId, Status> {
    let user_id = value.ok_or_else(|| Status::invalid_argument(format!("{field} is required")))?;
    if user_id.value.is_empty() {
      return Err(Status::invalid_argument(format!("{field} must not be empty")));
    }
    Ok(user_id)
  }

  fn next_message_id(messages: &[ChatMessage]) -> u64 {
    messages.iter().map(|message| message.id).max().unwrap_or(0) + 1
  }
}

#[tonic::async_trait]
impl ChatService for ChatServiceImpl {
  async fn send_message(
    &self,
    request: Request<SendMessageRequest>,
  ) -> Result<Response<SendMessageResponse>, Status> {
    let request = request.into_inner();
    let sender_id = Self::parse_user_id(request.sender_id, "sender_id")?;
    let receiver_id = Self::parse_user_id(request.receiver_id, "receiver_id")?;

    if request.content.is_empty() {
      return Err(Status::invalid_argument("content must not be empty"));
    }

    let key = Self::conversation_key(&sender_id.value, &receiver_id.value);
    let timestamp = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .map_err(|_| Status::internal("system clock is before the UNIX epoch"))?
      .as_millis() as i64;

    let mut storage = self
      .messages
      .lock()
      .map_err(|_| Status::internal("message storage lock poisoned"))?;

    let messages = storage.entry(key).or_default();
    let message = ChatMessage {
      id: Self::next_message_id(messages),
      sender_id: Some(sender_id),
      receiver_id: Some(receiver_id),
      content: request.content,
      timestamp,
    };
    messages.push(message.clone());

    Ok(Response::new(SendMessageResponse {
      message: Some(message),
    }))
  }

  async fn get_conversation(
    &self,
    request: Request<GetConversationRequest>,
  ) -> Result<Response<GetConversationResponse>, Status> {
    let request = request.into_inner();
    let user_id = Self::parse_user_id(request.user_id, "user_id")?;
    let other_user_id = Self::parse_user_id(request.other_user_id, "other_user_id")?;

    let key = Self::conversation_key(&user_id.value, &other_user_id.value);
    let storage = self
      .messages
      .lock()
      .map_err(|_| Status::internal("message storage lock poisoned"))?;

    let messages = storage.get(&key).cloned().unwrap_or_default();

    Ok(Response::new(GetConversationResponse { messages }))
  }
}
