use async_bulkhead::BulkheadError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BulkheadMiddlewareError {
    #[error("{0}")]
    BulkheadError(#[from] BulkheadError),
}

impl From<BulkheadMiddlewareError> for reqwest_middleware::Error {
    fn from(err: BulkheadMiddlewareError) -> reqwest_middleware::Error {
        reqwest_middleware::Error::Middleware(err.into())
    }
}
