use application::error::AppError;
use application::event::event_bus::EventEnvelope;
use application::event::event_bus::{ErasedHandler, EventBus, Handler};
use async_trait::async_trait;
use futures::future::join_all;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 桥接，将 Handler<E> 擦除为 ErasedHandler
pub struct HandlerWrapper<E> {
    inner: Arc<dyn Handler<E>>,
}

#[async_trait]
impl<E> ErasedHandler for HandlerWrapper<E>
where
    E: Send + Sync + 'static,
{
    async fn handle_erased(&self, event: &(dyn Any + Send + Sync)) {
        if let Some(e) = event.downcast_ref::<EventEnvelope<E>>() {
            self.inner.handle(e).await;
        }
    }
}

/// 内存事件总线
#[derive(Clone)]
pub struct InMemoryEventBus {
    handlers: Arc<RwLock<HashMap<TypeId, Vec<Arc<dyn ErasedHandler>>>>>,
    /// 是否异步触发处理器（不等待完成）
    fire_and_forget: bool,
}

impl InMemoryEventBus {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
            fire_and_forget: false,
        }
    }

    /// 创建一个异步触发的事件总线（不等待处理器完成）
    pub fn new_async() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
            fire_and_forget: true,
        }
    }
}

#[async_trait]
impl EventBus for InMemoryEventBus {
    async fn publish<E>(&self, event: EventEnvelope<E>) -> Result<(), AppError>
    where
        E: Send + Sync + 'static,
    {
        let type_id = TypeId::of::<E>();

        let handlers: Option<Vec<Arc<dyn ErasedHandler>>> = {
            let guard = self.handlers.read().await;
            guard.get(&type_id).cloned()
        };

        if let Some(list) = handlers {
            if self.fire_and_forget {
                // 异步触发：spawn单个任务处理所有handlers
                let event_arc = Arc::new(event);
                tokio::spawn(async move {
                    let futures = list.iter().map(|h| h.handle_erased(event_arc.as_ref()));
                    join_all(futures).await;
                });
            } else {
                // 同步等待所有处理器完成
                let futures = list.iter().map(|h| h.handle_erased(&event));
                join_all(futures).await;
            }
        }
        Ok(())
    }

    async fn subscribe<E>(&mut self, handler: Arc<dyn Handler<E>>)
    where
        E: Send + Sync + 'static,
    {
        //let type_id = TypeId::of::<E>();
        let wrapper = Arc::new(HandlerWrapper { inner: handler }) as Arc<dyn ErasedHandler>;
        self.handlers
            .write()
            .await
            .entry(TypeId::of::<E>())
            .or_default()
            .push(wrapper);
    }
}
