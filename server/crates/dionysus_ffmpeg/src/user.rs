use std::sync::Mutex;

use diesel::{PgConnection, result::Error as DieselError};
use dionysus_db::{
  create_user, find_user, is_username_conflict, list_user_friends,
  models::{NewUser, User, UserProfileChanges},
  update_user_profile, validate_profile,
};
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::proto::dionysus::user::v1::{
  AgeBracket, CreateProfileRequest, CreateProfileResponse, GetFriendListRequest,
  GetFriendListResponse, GetProfileRequest, GetProfileResponse, UpdateProfileRequest,
  UpdateProfileResponse, User as ProtoUser, UserId, user_service_server::UserService,
};

pub struct UserServiceImpl {
  connection: Mutex<PgConnection>,
}

impl UserServiceImpl {
  pub fn new(connection: PgConnection) -> Self {
    Self {
      connection: Mutex::new(connection),
    }
  }

  fn database_error(error: DieselError) -> Status {
    if is_username_conflict(&error) {
      Status::already_exists("username is already in use")
    } else {
      tracing::error!(?error, "database operation failed");
      Status::internal("database operation failed")
    }
  }

  fn parse_id(value: &str, field: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(value).map_err(|_| Status::invalid_argument(format!("{field} must be a UUID")))
  }

  fn request_user_id<T>(request: &Request<T>) -> Result<Uuid, Status> {
    let value = request
      .metadata()
      .get("user-id")
      .ok_or_else(|| Status::unauthenticated("missing user-id metadata"))?
      .to_str()
      .map_err(|_| Status::unauthenticated("user-id metadata is not valid ASCII"))?;
    Self::parse_id(value, "user-id")
  }

  fn to_proto(user: User) -> ProtoUser {
    ProtoUser {
      id: Some(UserId {
        value: user.id.to_string(),
      }),
      username: user.username.unwrap_or_default(),
      display_name: user.display_name,
      bio: user.bio.unwrap_or_default(),
      avatar_url: user.avatar_url.unwrap_or_default(),
      age: user.age_bracket,
    }
  }

  fn age_value(age: Option<i32>) -> Result<Option<i32>, Status> {
    let Some(value) = age else {
      return Ok(None);
    };
    let bracket = AgeBracket::try_from(value)
      .map_err(|_| Status::invalid_argument("age must be a valid AgeBracket"))?;
    Ok((bracket != AgeBracket::Unspecified).then_some(value))
  }
}

#[tonic::async_trait]
impl UserService for UserServiceImpl {
  async fn create_profile(
    &self,
    request: Request<CreateProfileRequest>,
  ) -> Result<Response<CreateProfileResponse>, Status> {
    let request = request.into_inner();
    let age = Self::age_value(request.age)?;
    validate_profile(
      Some(&request.username),
      Some(&request.display_name),
      Some(&request.bio),
      Some(&request.avatar_url),
      age,
    )
    .map_err(Status::invalid_argument)?;

    let mut connection = self
      .connection
      .lock()
      .map_err(|_| Status::internal("database connection lock poisoned"))?;
    let user = create_user(
      &mut connection,
      NewUser {
        display_name: &request.display_name,
        email: None,
        avatar_url: (!request.avatar_url.is_empty()).then_some(request.avatar_url.as_str()),
        username: (!request.username.is_empty()).then_some(request.username.as_str()),
        bio: (!request.bio.is_empty()).then_some(request.bio.as_str()),
        age_bracket: age,
      },
    )
    .map_err(Self::database_error)?;

    Ok(Response::new(CreateProfileResponse {
      user: Some(Self::to_proto(user)),
    }))
  }

  async fn get_profile(
    &self,
    request: Request<GetProfileRequest>,
  ) -> Result<Response<GetProfileResponse>, Status> {
    let id = request
      .into_inner()
      .user_id
      .ok_or_else(|| Status::invalid_argument("user_id is required"))?;
    let user_id = Self::parse_id(&id.value, "user_id")?;
    let mut connection = self
      .connection
      .lock()
      .map_err(|_| Status::internal("database connection lock poisoned"))?;
    let user = find_user(&mut connection, user_id)
      .map_err(Self::database_error)?
      .ok_or_else(|| Status::not_found("user not found"))?;
    Ok(Response::new(GetProfileResponse {
      user: Some(Self::to_proto(user)),
    }))
  }

  async fn update_profile(
    &self,
    request: Request<UpdateProfileRequest>,
  ) -> Result<Response<UpdateProfileResponse>, Status> {
    let user_id = Self::request_user_id(&request)?;
    let request = request.into_inner();
    let age = request
      .age
      .map(|value| Self::age_value(Some(value)))
      .transpose()?
      .flatten();
    validate_profile(
      request.username.as_deref(),
      request.display_name.as_deref(),
      request.bio.as_deref(),
      request.avatar_url.as_deref(),
      age,
    )
    .map_err(Status::invalid_argument)?;
    let changes = UserProfileChanges {
      username: request.username.as_deref(),
      display_name: request.display_name.as_deref(),
      bio: request.bio.as_deref(),
      avatar_url: request.avatar_url.as_deref(),
      age_bracket: request.age.map(|_| age).flatten(),
    };
    let mut connection = self
      .connection
      .lock()
      .map_err(|_| Status::internal("database connection lock poisoned"))?;
    let user =
      update_user_profile(&mut connection, user_id, changes).map_err(Self::database_error)?;
    Ok(Response::new(UpdateProfileResponse {
      user: Some(Self::to_proto(user)),
    }))
  }

  async fn get_friend_list(
    &self,
    request: Request<GetFriendListRequest>,
  ) -> Result<Response<GetFriendListResponse>, Status> {
    let id = request
      .into_inner()
      .user_id
      .ok_or_else(|| Status::invalid_argument("user_id is required"))?;
    let user_id = Self::parse_id(&id.value, "user_id")?;
    let mut connection = self
      .connection
      .lock()
      .map_err(|_| Status::internal("database connection lock poisoned"))?;
    let friends = list_user_friends(&mut connection, user_id).map_err(Self::database_error)?;
    Ok(Response::new(GetFriendListResponse {
      friends: friends.into_iter().map(Self::to_proto).collect(),
    }))
  }
}
