//! The public CRM boundary. Other modules cannot access CRM's stores or tables.
use async_trait::async_trait;
use bqatlas_core::AppError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerSummary {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub active: bool,
}
#[async_trait]
pub trait CustomerDirectory: Send + Sync {
    async fn find(&self, id: Uuid) -> Result<Option<CustomerSummary>, AppError>;
}
pub const CUSTOMER_LOOKUP: &str = "crm.customers.lookup";
