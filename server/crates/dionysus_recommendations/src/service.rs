use std::sync::{Mutex, MutexGuard};

use chrono::{Duration, Utc};
use diesel::PgConnection;
use dionysus_db::recommendations::{
  ScoredMedia, cached_suggestions, for_profile_by_cooccurrence, profile_id_for_user, record_event,
  similar_by_cooccurrence, similar_by_embedding, trending, upsert_rating,
};
use dionysus_proto::dionysus::{
  common::v1::Uuid as ProtoUuid,
  recommendations::v1::{
    GetForUserRequest, GetForUserResponse, GetForYouRequest, GetForYouResponse, GetSimilarRequest,
    GetSimilarResponse, GetTrendingRequest, GetTrendingResponse,
    Recommendation as ProtoRecommendation, RecordEventRequest, RecordEventResponse,
    RecordRatingRequest, RecordRatingResponse,
    recommendation_service_server::RecommendationService,
  },
};
use tonic::{Request, Response, Status};
use uuid::Uuid;

const MAX_LIMIT: u32 = 100;
const TRENDING_WINDOW_DAYS: i64 = 30;
const EVENT_KINDS: [&str; 4] = ["impression", "click", "play", "dismiss"];

pub struct RecommendationServiceImpl {
  connection: Mutex<PgConnection>,
  locale: String,
}

impl RecommendationServiceImpl {
  pub fn new(connection: PgConnection) -> Self {
    let locale = std::env::var("RECOMMENDATION_LOCALE").unwrap_or_else(|_| "en".to_string());
    Self {
      connection: Mutex::new(connection),
      locale,
    }
  }

  fn connection(&self) -> Result<MutexGuard<'_, PgConnection>, Status> {
    self
      .connection
      .lock()
      .map_err(|_| Status::internal("database connection lock poisoned"))
  }

  fn database_error(error: diesel::result::Error) -> Status {
    tracing::error!(?error, "recommendation query failed");
    Status::internal("recommendation query failed")
  }

  fn id_from_proto(id: Option<ProtoUuid>, field: &str) -> Result<Uuid, Status> {
    let bytes = id
      .map(|id| id.value)
      .ok_or_else(|| Status::invalid_argument(format!("{field} is required")))?;
    Uuid::from_slice(&bytes)
      .map_err(|_| Status::invalid_argument(format!("{field} must be a 16-byte UUID")))
  }

  fn id_to_proto(id: Uuid) -> ProtoUuid {
    ProtoUuid {
      value: id.as_bytes().to_vec(),
    }
  }

  fn limit(value: u32) -> i64 {
    value.clamp(1, MAX_LIMIT) as i64
  }

  fn to_proto(row: ScoredMedia, reason: &str, source: Option<Uuid>) -> ProtoRecommendation {
    ProtoRecommendation {
      media_id: Some(Self::id_to_proto(row.media_id)),
      title: row.title,
      score: row.score,
      reason: reason.to_string(),
      source_media_id: source.map(Self::id_to_proto),
    }
  }

  fn for_you_items(
    &self,
    connection: &mut PgConnection,
    profile_id: Uuid,
    limit: i64,
  ) -> Result<Vec<ProtoRecommendation>, Status> {
    let cached =
      cached_suggestions(connection, profile_id, "for_you", limit).map_err(Self::database_error)?;
    if !cached.is_empty() {
      return Ok(
        cached
          .into_iter()
          .map(|row| Self::to_proto(row, "for_you", None))
          .collect(),
      );
    }
    // Cold cache: co-occurrence over this profile's own history.
    Ok(
      for_profile_by_cooccurrence(connection, profile_id, limit)
        .map_err(Self::database_error)?
        .into_iter()
        .map(|row| Self::to_proto(row, "because_you_watched", None))
        .collect(),
    )
  }
}

#[tonic::async_trait]
impl RecommendationService for RecommendationServiceImpl {
  async fn get_trending(
    &self,
    request: Request<GetTrendingRequest>,
  ) -> Result<Response<GetTrendingResponse>, Status> {
    let limit = Self::limit(request.into_inner().limit);
    let mut connection = self.connection()?;
    let rows = trending(
      &mut connection,
      Utc::now() - Duration::days(TRENDING_WINDOW_DAYS),
      limit,
    )
    .map_err(Self::database_error)?;

    Ok(Response::new(GetTrendingResponse {
      items: rows
        .into_iter()
        .map(|row| Self::to_proto(row, "trending", None))
        .collect(),
    }))
  }

  async fn get_for_you(
    &self,
    request: Request<GetForYouRequest>,
  ) -> Result<Response<GetForYouResponse>, Status> {
    let request = request.into_inner();
    let profile_id = Self::id_from_proto(request.profile_id, "profile_id")?;
    let limit = Self::limit(request.limit);
    let mut connection = self.connection()?;
    let items = self.for_you_items(&mut connection, profile_id, limit)?;

    Ok(Response::new(GetForYouResponse { items }))
  }

  async fn get_for_user(
    &self,
    request: Request<GetForUserRequest>,
  ) -> Result<Response<GetForUserResponse>, Status> {
    let request = request.into_inner();
    let user_id = Self::id_from_proto(request.user_id, "user_id")?;
    let limit = Self::limit(request.limit);
    let mut connection = self.connection()?;
    let profile_id = profile_id_for_user(&mut connection, user_id)
      .map_err(Self::database_error)?
      .ok_or_else(|| Status::not_found("no viewer profile for this user"))?;
    let items = self.for_you_items(&mut connection, profile_id, limit)?;

    Ok(Response::new(GetForUserResponse { items }))
  }

  async fn get_similar(
    &self,
    request: Request<GetSimilarRequest>,
  ) -> Result<Response<GetSimilarResponse>, Status> {
    let request = request.into_inner();
    let media_id = Self::id_from_proto(request.media_id, "media_id")?;
    let limit = Self::limit(request.limit);
    let mut connection = self.connection()?;

    let mut rows = similar_by_embedding(&mut connection, media_id, &self.locale, limit)
      .map_err(Self::database_error)?;
    let reason = if rows.is_empty() {
      rows =
        similar_by_cooccurrence(&mut connection, media_id, limit).map_err(Self::database_error)?;
      "because_you_watched"
    } else {
      "similar"
    };

    Ok(Response::new(GetSimilarResponse {
      items: rows
        .into_iter()
        .map(|row| Self::to_proto(row, reason, Some(media_id)))
        .collect(),
    }))
  }

  async fn record_rating(
    &self,
    request: Request<RecordRatingRequest>,
  ) -> Result<Response<RecordRatingResponse>, Status> {
    let request = request.into_inner();
    let profile_id = Self::id_from_proto(request.profile_id, "profile_id")?;
    let media_id = Self::id_from_proto(request.media_id, "media_id")?;
    if !(1..=5).contains(&request.value) {
      return Err(Status::invalid_argument("value must be between 1 and 5"));
    }
    let mut connection = self.connection()?;
    upsert_rating(&mut connection, profile_id, media_id, request.value as i16)
      .map_err(Self::database_error)?;
    Ok(Response::new(RecordRatingResponse {}))
  }

  async fn record_event(
    &self,
    request: Request<RecordEventRequest>,
  ) -> Result<Response<RecordEventResponse>, Status> {
    let request = request.into_inner();
    let profile_id = Self::id_from_proto(request.profile_id, "profile_id")?;
    let media_id = Self::id_from_proto(request.media_id, "media_id")?;
    if !EVENT_KINDS.contains(&request.kind.as_str()) {
      return Err(Status::invalid_argument(format!(
        "kind must be one of {}",
        EVENT_KINDS.join(", ")
      )));
    }
    let mut connection = self.connection()?;
    record_event(&mut connection, profile_id, media_id, &request.kind)
      .map_err(Self::database_error)?;
    Ok(Response::new(RecordEventResponse {}))
  }
}
