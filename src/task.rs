use crate::Context;
use anyhow::Result;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub struct Task {
    pub name: String,
    pub func:
        Arc<dyn Fn(Context) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync>,
    pub pre_condition:
        Arc<dyn Fn(Context) -> Pin<Box<dyn Future<Output = Result<bool>> + Send>> + Send + Sync>,
    pub repeatable: bool,
}

impl Clone for Task {
    fn clone(&self) -> Self {
        Task {
            name: self.name.clone(),
            func: Arc::clone(&self.func),
            pre_condition: Arc::clone(&self.pre_condition),
            repeatable: self.repeatable,
        }
    }
}

impl Task {
    pub fn new<F, P>(name: &str, func: F, pre_condition: P, repeatable: bool) -> Self
    where
        F: Fn(Context) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync + 'static,
        P: Fn(Context) -> Pin<Box<dyn Future<Output = Result<bool>> + Send>>
            + Send
            + Sync
            + 'static,
    {
        Task {
            name: name.to_string(),
            func: Arc::new(func),
            pre_condition: Arc::new(pre_condition),
            repeatable,
        }
    }
}

impl Task {
    pub fn new_async<F, Fut, P, Pout>(
        name: &str,
        func: F,
        pre_condition: P,
        repeatable: bool,
    ) -> Self
    where
        F: Fn(Context) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
        P: Fn(Context) -> Pout + Send + Sync + 'static,
        Pout: Future<Output = Result<bool>> + Send + 'static,
    {
        Task {
            name: name.to_string(),
            func: Arc::new(move |ctx| Box::pin(func(ctx))),
            pre_condition: Arc::new(move |ctx| Box::pin(pre_condition(ctx))),
            repeatable,
        }
    }
}
