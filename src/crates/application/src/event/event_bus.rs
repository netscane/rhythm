use crate::error::AppError;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::event::DomainEvent;
use std::any::Any;
use std::sync::Arc;
use uuid::Uuid;

/// 事件信封
#[derive(Debug, Clone)]
pub struct EventEnvelope<T> {
    pub id: EventId,
    pub aggregate_id: i64,
    pub version: i64,
    pub timestamp: DateTime<Utc>,
    pub payload: T,
    // correlation_id is used to trace a request
    pub correlation_id: CorrelationId,
    // causation_id is used to trace a causation, which is the event id of the event that caused this event
    pub causation_id: EventId,
}

impl<T> EventEnvelope<T> {
    pub fn new(
        aggregate_id: i64,
        version: i64,
        payload: T,
        correlation_id: CorrelationId,
        causation_id: EventId,
    ) -> Self {
        Self {
            id: EventId::new(),
            aggregate_id,
            version,
            timestamp: Utc::now(),
            payload,
            correlation_id,
            causation_id,
        }
    }

    pub fn new_with_domain_event<E: DomainEvent>(
        event: E,
        correlation_id: CorrelationId,
        causation_id: EventId,
    ) -> EventEnvelope<E> {
        EventEnvelope {
            id: EventId::new(),
            aggregate_id: event.aggregate_id(),
            version: event.version(),
            timestamp: Utc::now(),
            payload: event,
            correlation_id,
            causation_id,
        }
    }

    pub fn with_correlation_id(mut self, correlation_id: CorrelationId) -> Self {
        self.correlation_id = correlation_id;
        self
    }
    pub fn with_causation_id(mut self, causation_id: EventId) -> Self {
        self.causation_id = causation_id;
        self
    }
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct EventId(Uuid);

impl EventId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct CorrelationId(Uuid);

impl CorrelationId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

/// 强类型 Handler
#[async_trait]
pub trait Handler<E>: Send + Sync {
    async fn handle(&self, event: &EventEnvelope<E>);
}

/// 类型擦除 Handler，用 Any 做事件擦除
#[async_trait]
pub trait ErasedHandler: Send + Sync {
    async fn handle_erased(&self, event: &(dyn Any + Send + Sync));
}

/// 事件总线抽象
#[async_trait]
pub trait EventBus: Send + Sync {
    async fn publish<E>(&self, event: EventEnvelope<E>) -> Result<(), AppError>
    where
        E: Send + Sync + 'static;

    async fn subscribe<E>(&mut self, handler: Arc<dyn Handler<E>>)
    where
        E: Send + Sync + 'static;
}
