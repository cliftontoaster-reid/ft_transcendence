use std::{collections::HashSet, sync::Mutex};

use diesel::{PgConnection, result::Error as DieselError};
use dionysus_db::{
  create_user, find_user, find_user_by_username, find_users_by_ids, is_username_conflict,
  list_user_friends,
  models::{NewUser, User, UserProfileChanges},
  update_user_profile, username_exists, validate_profile,
};
use tonic::{Request, Response, Status};
use uuid::Uuid;

use dionysus_proto::dionysus::{
  common::v1::Uuid as ProtoUuid,
  user::v1::{
    AgeBracket, BatchGetProfilesRequest, BatchGetProfilesResponse,
    CheckUsernameAvailabilityRequest, CheckUsernameAvailabilityResponse, CreateProfileRequest,
    CreateProfileResponse, GetFriendListRequest, GetFriendListResponse,
    GetProfileByUsernameRequest, GetProfileByUsernameResponse, GetProfileRequest,
    GetProfileResponse, UpdateProfileRequest, UpdateProfileResponse, User as ProtoUser, UserId,
    user_service_server::UserService,
  },
};

/// Upper bound documented on `BatchGetProfilesRequest.user_ids`.
const MAX_BATCH_PROFILES: usize = 100;

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

  /// `UserID { value: UUID { value: bytes } }` -> `Uuid` (16 raw bytes).
  fn id_from_proto(id: Option<UserId>, field: &str) -> Result<Uuid, Status> {
    let bytes = id
      .and_then(|id| id.value)
      .ok_or_else(|| Status::invalid_argument(format!("{field} is required")))?
      .value;
    Uuid::from_slice(&bytes)
      .map_err(|_| Status::invalid_argument(format!("{field} must be a 16-byte UUID")))
  }

  fn id_to_proto(id: Uuid) -> UserId {
    UserId {
      value: Some(ProtoUuid {
        value: id.as_bytes().to_vec(),
      }),
    }
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
      id: Some(Self::id_to_proto(user.id)),
      username: user.username.unwrap_or_default(),
      display_name: user.display_name,
      bio: user.bio.unwrap_or_default(),
      avatar_url: user.avatar_url.unwrap_or_default(),
      age: user.age_bracket,
    }
  }

  fn require_username(username: &str) -> Result<(), Status> {
    validate_profile(Some(username), None, None, None, None).map_err(Status::invalid_argument)
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
    let user_id = Self::id_from_proto(request.into_inner().user_id, "user_id")?;
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

  async fn get_profile_by_username(
    &self,
    request: Request<GetProfileByUsernameRequest>,
  ) -> Result<Response<GetProfileByUsernameResponse>, Status> {
    let username = request.into_inner().username;
    Self::require_username(&username)?;
    let mut connection = self
      .connection
      .lock()
      .map_err(|_| Status::internal("database connection lock poisoned"))?;
    let user = find_user_by_username(&mut connection, &username)
      .map_err(Self::database_error)?
      .ok_or_else(|| Status::not_found("user not found"))?;
    Ok(Response::new(GetProfileByUsernameResponse {
      user: Some(Self::to_proto(user)),
    }))
  }

  async fn batch_get_profiles(
    &self,
    request: Request<BatchGetProfilesRequest>,
  ) -> Result<Response<BatchGetProfilesResponse>, Status> {
    let request = request.into_inner();
    if request.user_ids.len() > MAX_BATCH_PROFILES {
      return Err(Status::invalid_argument(format!(
        "user_ids must contain at most {MAX_BATCH_PROFILES} entries"
      )));
    }
    let mut seen = HashSet::new();
    let mut ids = Vec::with_capacity(request.user_ids.len());
    for id in request.user_ids {
      let id = Self::id_from_proto(Some(id), "user_ids")?;
      if seen.insert(id) {
        ids.push(id);
      }
    }
    if ids.is_empty() {
      return Ok(Response::new(BatchGetProfilesResponse {
        users: Vec::new(),
      }));
    }
    let mut connection = self
      .connection
      .lock()
      .map_err(|_| Status::internal("database connection lock poisoned"))?;
    let users = find_users_by_ids(&mut connection, &ids).map_err(Self::database_error)?;
    Ok(Response::new(BatchGetProfilesResponse {
      users: users.into_iter().map(Self::to_proto).collect(),
    }))
  }

  async fn check_username_availability(
    &self,
    request: Request<CheckUsernameAvailabilityRequest>,
  ) -> Result<Response<CheckUsernameAvailabilityResponse>, Status> {
    let username = request.into_inner().username;
    Self::require_username(&username)?;
    let mut connection = self
      .connection
      .lock()
      .map_err(|_| Status::internal("database connection lock poisoned"))?;
    let taken = username_exists(&mut connection, &username).map_err(Self::database_error)?;
    Ok(Response::new(CheckUsernameAvailabilityResponse {
      available: !taken,
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
      age_bracket: age,
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
    let user_id = Self::id_from_proto(request.into_inner().user_id, "user_id")?;
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
