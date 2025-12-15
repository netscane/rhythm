pub mod in_memory;
use application::error::AppError;
use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::any::type_name;

pub fn stable_event_type<E: 'static>() -> String {
    type_name::<E>().to_string()
}

/// Event envelope with common metadata. Keep your domain event as the payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope<E> {
    pub id: String,                     // ULID/UUID
    pub occurred_at: DateTime<Utc>,     // event time
    pub aggregate: String,              // e.g. "Library", "Album"
    pub aggregate_id: String,           // your aggregate id
    pub event_type: String,             // stable event type name (topic key)
    pub correlation_id: Option<String>, // for tracing a request
    pub causation_id: Option<String>,   // which event/command caused this
    pub payload: E,                     // the real domain event
}

impl<E> EventEnvelope<E>
where
    E: Serialize + DeserializeOwned + 'static,
{
    pub fn new(
        id: impl Into<String>,
        aggregate: impl Into<String>,
        aggregate_id: impl Into<String>,
        payload: E,
    ) -> Self {
        Self {
            id: id.into(),
            occurred_at: Utc::now(),
            aggregate: aggregate.into(),
            aggregate_id: aggregate_id.into(),
            event_type: stable_event_type::<E>(),
            correlation_id: None,
            causation_id: None,
            payload,
        }
    }
}
/// Codec abstraction (so Kafka/in-memory can share logic)
pub trait EventCodec: Send + Sync + 'static {
    fn encode<E: Serialize>(&self, env: &EventEnvelope<E>) -> Result<Vec<u8>, AppError>;
    fn decode<E: DeserializeOwned>(&self, bytes: &[u8]) -> Result<EventEnvelope<E>, AppError>;
}

/// Topic mapper: map an event type to a topic string. Customize if needed.
#[derive(Clone, Default)]
pub struct TopicMapper;
impl TopicMapper {
    pub fn topic_for<E: 'static>(&self) -> String {
        stable_event_type::<E>()
    }
}

/// Default JSON codec using `serde_json`.
#[derive(Clone, Default)]
pub struct JsonCodec;

impl EventCodec for JsonCodec {
    fn encode<E>(&self, env: &EventEnvelope<E>) -> Result<Vec<u8>, AppError>
    where
        E: Serialize,
    {
        serde_json::to_vec(env).map_err(|e| AppError::UnknownError(e.to_string()))
    }
    fn decode<E: DeserializeOwned>(&self, bytes: &[u8]) -> Result<EventEnvelope<E>, AppError> {
        serde_json::from_slice(bytes).map_err(|e| AppError::UnknownError(e.to_string()))
    }
}
