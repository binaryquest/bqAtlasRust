//! Explicit, development-only demo data. Stable IDs keep reruns from replacing edits.
use bqatlas_core::AppError;
use bqatlas_db::Database;
use bqatlas_sample_crm::{CustomerInput, CustomerService};
use bqatlas_sample_engagement::{EngagementService, Kind};
use bqatlas_sample_sales::{QuoteInput, QuoteLineInput, QuoteService};
use chrono::{Days, Utc};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;
fn id(key: &str) -> Uuid {
    Uuid::new_v5(
        &Uuid::NAMESPACE_URL,
        format!("https://binaryquest.com/bqatlas-rust/demo/{key}").as_bytes(),
    )
}
pub async fn seed(db: Database, environment: &str) -> Result<(), AppError> {
    if environment != "Development" {
        return Err(AppError::Forbidden);
    }
    let crm = Arc::new(CustomerService::new(db.clone()));
    let engagement = EngagementService::new(db.clone(), crm.clone());
    let quotes = QuoteService::new(db, crm.clone());
    let mut customers = vec![];
    for (code, name, email) in [
        ("DEMO-NORTH", "Northwind Services", "alex@northwind.example"),
        ("DEMO-ALPINE", "Alpine Studio", "morgan@alpine.example"),
        ("DEMO-SUMMIT", "Summit Manufacturing", "sam@summit.example"),
        ("DEMO-HARBOR", "Harbor Logistics", "lee@harbor.example"),
    ] {
        let customer = crm
            .seed_customer(CustomerInput {
                code: code.into(),
                name: name.into(),
                email: Some(email.into()),
                active: true,
            })
            .await?;
        customers.push(customer);
    }
    for (code, name, category, price) in [
        (
            "DEMO-CONSULT",
            "Implementation consulting",
            "Service",
            "125.0000",
        ),
        (
            "DEMO-TRAIN",
            "Team training workshop",
            "Service",
            "500.0000",
        ),
        (
            "DEMO-SUPPORT",
            "Priority support plan",
            "Service",
            "250.0000",
        ),
        (
            "DEMO-SCANNER",
            "Warehouse barcode scanner",
            "Stock",
            "189.9500",
        ),
        ("DEMO-LABEL", "Shipping label pack", "Stock", "24.5000"),
    ] {
        engagement.seed(Kind::Products,id(code),json!({"code":code,"name":name,"category":category,"unitPrice":price,"currency":"USD","active":true})).await?;
    }
    let today = Utc::now().date_naive();
    for (name, stage, amount, index) in [
        ("Warehouse rollout", "Qualified", "8400.00", 0),
        ("Customer portal launch", "Proposal", "12500.00", 1),
        ("Support renewal", "Won", "3000.00", 0),
        ("Production planning", "Lead", "18000.00", 2),
        ("Dispatch modernization", "Qualified", "6200.00", 3),
        ("Legacy platform replacement", "Lost", "4500.00", 1),
    ] {
        let customer = &customers[index];
        engagement.seed(Kind::Opportunities,id(name),json!({"name":name,"customerId":customer.id,"stage":stage,"amount":amount,"currency":"USD","expectedClose":today+Days::new(14+index as u64*7),"owner":if index%2==0{"Alex Morgan"}else{"Jamie Lee"},"notes":"Demo opportunity. Change stage, value and next steps."})).await?;
    }
    for (name, kind, status, index, days) in [
        ("Confirm rollout requirements", "Call", "Planned", 0, -1),
        ("Present portal proposal", "Meeting", "Planned", 1, 1),
        ("Send production discovery notes", "Email", "Done", 2, 0),
        ("Book dispatch workshop", "Task", "Planned", 3, 3),
        ("Review renewal success", "Call", "Planned", 0, 7),
    ] {
        let customer = &customers[index];
        engagement.seed(Kind::Activities,id(name),json!({"name":name,"customerId":customer.id,"kind":kind,"status":status,"dueDate":today+chrono::Duration::days(days),"owner":"Alex Morgan","notes":"Capture the outcome here and mark Done when completed."})).await?;
    }
    for customer in customers.iter().take(3) {
        quotes
            .seed(
                id(&format!("quote:{}", customer.code)),
                QuoteInput {
                    customer_id: customer.id,
                    date: today,
                    currency: "USD".into(),
                    lines: vec![
                        QuoteLineInput {
                            id: id("quote-line-consult"),
                            description: "Implementation consulting".into(),
                            quantity: "8".into(),
                            unit_price: "125.0000".into(),
                        },
                        QuoteLineInput {
                            id: id("quote-line-training"),
                            description: "Team training workshop".into(),
                            quantity: "1".into(),
                            unit_price: "500.0000".into(),
                        },
                    ],
                },
            )
            .await?;
    }
    Ok(())
}
