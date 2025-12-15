use crate::event::event_bus::EventId;
use crate::event::event_bus::{CorrelationId, EventEnvelope};
#[derive(Debug, Clone)]
pub struct AppContext {
    pub event_id: EventId,
    pub correlation_id: CorrelationId,
    pub causation_id: EventId,
}

impl AppContext {
    pub fn new() -> Self {
        let event_id = EventId::new();
        Self {
            event_id: event_id.clone(),
            correlation_id: CorrelationId::new(),
            causation_id: event_id,
        }
    }
    pub fn inherit_from(parent: &AppContext) -> Self {
        parent.inherit()
    }
    pub fn inherit(&self) -> Self {
        Self {
            event_id: EventId::new(),
            correlation_id: self.correlation_id.clone(),
            causation_id: self.event_id.clone(),
        }
    }
}

impl<T> From<&EventEnvelope<T>> for AppContext {
    fn from(envelope: &EventEnvelope<T>) -> Self {
        Self {
            event_id: envelope.id.clone(),
            correlation_id: envelope.correlation_id.clone(),
            causation_id: envelope.causation_id.clone(),
        }
    }
}
