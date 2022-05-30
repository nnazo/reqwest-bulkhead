use crate::error::BulkheadMiddlewareError;
use anyhow::anyhow;
use async_bulkhead::{Bulkhead, BulkheadRegistry};
use reqwest::{Request, Response};
use reqwest_middleware::{Error, Middleware, Next, Result};
use task_local_extensions::Extensions;

/// A middleware for limiting calls using the provided bulkhead
pub struct BulkheadMiddleware {
    bulkhead: Bulkhead,
}

impl BulkheadMiddleware {
    pub fn new(bulkhead: Bulkhead) -> Self {
        Self::from(bulkhead)
    }
}

impl From<Bulkhead> for BulkheadMiddleware {
    fn from(bulkhead: Bulkhead) -> BulkheadMiddleware {
        BulkheadMiddleware { bulkhead }
    }
}

#[async_trait::async_trait]
impl Middleware for BulkheadMiddleware {
    async fn handle(
        &self,
        req: Request,
        extensions: &mut Extensions,
        next: Next<'_>,
    ) -> Result<Response> {
        limit_call(&self.bulkhead, req, extensions, next).await
    }
}

/// A middleware for limiting calls using the bulkhead corresponding to
/// the URL host inside of the provided [`BulkheadRegistry`]
pub struct BulkheadRegistryMiddleware {
    registry: BulkheadRegistry,
}

impl BulkheadRegistryMiddleware {
    pub fn new(registry: BulkheadRegistry) -> Self {
        Self::from(registry)
    }
}

impl From<BulkheadRegistry> for BulkheadRegistryMiddleware {
    fn from(registry: BulkheadRegistry) -> BulkheadRegistryMiddleware {
        BulkheadRegistryMiddleware { registry }
    }
}

#[async_trait::async_trait]
impl Middleware for BulkheadRegistryMiddleware {
    async fn handle(
        &self,
        req: Request,
        extensions: &mut Extensions,
        next: Next<'_>,
    ) -> Result<Response> {
        let host = req
            .url()
            .host_str()
            .ok_or_else(|| Error::Middleware(anyhow!("Request did not have a valid base URL.")))?;
        let bulkhead = self.registry.get(host).ok_or_else(|| {
            Error::Middleware(anyhow!(
                "Bulkhead registry did not contain bulkhead for resource `{}`",
                host
            ))
        })?;
        limit_call(bulkhead, req, extensions, next).await
    }
}

async fn limit_call(
    bulkhead: &Bulkhead,
    req: Request,
    extensions: &mut Extensions,
    next: Next<'_>,
) -> Result<Response> {
    bulkhead
        .limit(next.run(req, extensions))
        .await
        .map_err(|e| {
            tracing::warn!("Bulkhead limited resource call: {:?}", e);
            BulkheadMiddlewareError::from(e)
        })?
}
