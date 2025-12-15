pub trait DomainEvent: Send + Sync {
    fn aggregate_id(&self) -> i64;
    fn version(&self) -> i64;
}
