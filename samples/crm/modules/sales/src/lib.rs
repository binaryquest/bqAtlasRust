//! Sales aggregate. Header, child lines, concurrency and audit share one transaction.
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::Response,
    routing::{get, post},
};
use bqatlas_contracts::{
    FieldDescriptor, PageResult, QueryRequest, RecordResult, ResourceDescriptor,
};
use bqatlas_core::{
    AppError, ModuleDefinition, ValidationErrors, decimal_text, expected_version, invalid,
    new_version, search_key, validate_query,
};
use bqatlas_db::{Database, DbRow, Migration, Sql, Transaction, Value};
use bqatlas_http::ApiPath as Path;
use bqatlas_http::{ApiJson, ApiResult, CurrentActor, record};
use bqatlas_sample_crm_contracts::{CUSTOMER_LOOKUP, CustomerDirectory};
use chrono::{DateTime, Datelike, NaiveDate, SubsecRound, Utc};
use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, sync::Arc};
use uuid::Uuid;
pub const PERMISSIONS: [&str; 4] = [
    "sales.quotes.read",
    "sales.quotes.write",
    "sales.quotes.delete",
    "sales.quotes.submit",
];
pub fn definition() -> ModuleDefinition {
    ModuleDefinition {
        id: "sales".into(),
        dependencies: vec!["crm".into()],
    }
}
pub fn descriptor() -> ResourceDescriptor {
    ResourceDescriptor {
        id: "sales.quotes".into(),
        title: "Sales quotes".into(),
        endpoint: "/api/v1/sales/quotes".into(),
        read_permission: PERMISSIONS[0].into(),
        fields: vec![
            FieldDescriptor::new("customerId", "Customer", "string", true, None),
            FieldDescriptor::new("date", "Date", "date", true, None),
            FieldDescriptor::new("currency", "Currency", "string", true, Some(3)),
        ],
        key_field: "id".into(),
        o_data_endpoint: Some("/odata/Quotes".into()),
    }
}
pub fn migration() -> Migration {
    Migration {
        module: "sales",
        version: 1,
        sql: Sql::new(
            include_str!("../migrations/postgresql/0001.sql"),
            include_str!("../migrations/sqlserver/0001.sql"),
        ),
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteLineInput {
    pub id: Uuid,
    pub description: String,
    pub quantity: String,
    pub unit_price: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteInput {
    pub customer_id: Uuid,
    pub date: NaiveDate,
    pub currency: String,
    pub lines: Vec<QuoteLineInput>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteLineDto {
    pub id: Uuid,
    pub description: String,
    pub quantity: String,
    pub unit_price: String,
    pub total: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteDto {
    pub id: Uuid,
    pub number: String,
    pub customer_id: Uuid,
    pub customer_code: String,
    pub customer_name: String,
    pub date: NaiveDate,
    pub currency: String,
    pub status: String,
    pub lines: Vec<QuoteLineDto>,
    pub total: String,
    pub modified_at: DateTime<Utc>,
    pub submitted_at: Option<DateTime<Utc>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteSummary {
    pub id: Uuid,
    pub number: String,
    pub customer_name: String,
    pub date: NaiveDate,
    pub currency: String,
    pub status: String,
    pub total: String,
}
struct ValidatedLine {
    id: Uuid,
    description: String,
    quantity: Decimal,
    unit_price: Decimal,
    total: Decimal,
}
struct ValidatedQuote {
    customer_id: Uuid,
    date: NaiveDate,
    currency: String,
    lines: Vec<ValidatedLine>,
    total: Decimal,
}
fn validate(input: QuoteInput) -> Result<ValidatedQuote, AppError> {
    let mut errors = ValidationErrors::new();
    let currency = input.currency.trim().to_ascii_uppercase();
    if input.customer_id.is_nil() {
        errors.insert("customerId".into(), vec!["Choose a customer.".into()]);
    }
    if input.date.year() < 1900 {
        errors.insert(
            "date".into(),
            vec!["Choose a date from 1900 onwards.".into()],
        );
    }
    if !matches!(currency.as_str(), "USD" | "EUR" | "BDT") {
        errors.insert("currency".into(), vec!["Choose USD, EUR or BDT.".into()]);
    }
    if input.lines.is_empty() || input.lines.len() > 100 {
        errors.insert(
            "lines".into(),
            vec!["A quote needs between 1 and 100 lines.".into()],
        );
    }
    let mut ids = BTreeSet::new();
    let mut lines = vec![];
    let mut total = Decimal::ZERO;
    for (index, line) in input.lines.into_iter().take(100).enumerate() {
        let prefix = format!("lines[{index}]");
        if line.id.is_nil() || !ids.insert(line.id) {
            errors.insert(
                format!("{prefix}.id"),
                vec!["Each line needs a unique nonempty ID.".into()],
            );
        }
        let description = line.description.trim().to_owned();
        if description.is_empty() || description.encode_utf16().count() > 200 {
            errors.insert(
                format!("{prefix}.description"),
                vec!["Use a description of 1–200 characters.".into()],
            );
        }
        let quantity = decimal_text(&line.quantity, 3, Decimal::from(1_000_000))
            .filter(|v| *v > Decimal::ZERO);
        let price = decimal_text(&line.unit_price, 4, Decimal::from(1_000_000_000));
        if quantity.is_none() {
            errors.insert(
                format!("{prefix}.quantity"),
                vec!["Use a positive decimal up to 1000000 with at most 3 decimal places.".into()],
            );
        }
        if price.is_none() {
            errors.insert(
                format!("{prefix}.unitPrice"),
                vec!["Use a decimal from 0 to 1000000000 with at most 4 decimal places.".into()],
            );
        }
        if let (Some(quantity), Some(unit_price)) = (quantity, price) {
            let value = quantity
                .checked_mul(unit_price)
                .ok_or(AppError::Internal)?
                .round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero);
            total = total.checked_add(value).ok_or(AppError::Internal)?;
            lines.push(ValidatedLine {
                id: line.id,
                description,
                quantity,
                unit_price,
                total: value,
            });
        }
    }
    if !errors.is_empty() {
        return Err(AppError::Validation(errors));
    }
    Ok(ValidatedQuote {
        customer_id: input.customer_id,
        date: input.date,
        currency,
        lines,
        total,
    })
}
fn conflict(code: &str, message: &str) -> AppError {
    AppError::Conflict {
        code: code.into(),
        message: message.into(),
    }
}
fn money(value: Decimal) -> String {
    format!("{value:.2}")
}
fn summary(row: &DbRow) -> Result<QuoteSummary, AppError> {
    Ok(QuoteSummary {
        id: row.uuid("id").map_err(|e| e.application())?,
        number: row.text("number").map_err(|e| e.application())?,
        customer_name: row.text("customer_name").map_err(|e| e.application())?,
        date: row.date("day").map_err(|e| e.application())?,
        currency: row.text("currency").map_err(|e| e.application())?,
        status: row.text("status").map_err(|e| e.application())?,
        total: money(row.decimal("total").map_err(|e| e.application())?),
    })
}
#[derive(Clone)]
pub struct QuoteService {
    db: Database,
    customers: Arc<dyn CustomerDirectory>,
}
impl QuoteService {
    pub fn new(db: Database, customers: Arc<dyn CustomerDirectory>) -> Self {
        Self { db, customers }
    }
    pub async fn get(&self, id: Uuid) -> Result<Option<RecordResult<QuoteDto>>, AppError> {
        let mut tx = self.db.begin().await.map_err(|e| e.application())?;
        let rows = tx
            .query(
                Sql::new(
                    "SELECT * FROM sales.quotes WHERE id=$1 FOR SHARE",
                    "SELECT * FROM sales.quotes WITH (HOLDLOCK) WHERE id=@P1",
                ),
                &[Value::Uuid(id)],
            )
            .await
            .map_err(|e| e.application())?;
        let Some(row) = rows.first() else {
            return Ok(None);
        };
        let q = summary(row)?;
        let rows_lines = tx
            .query(
                Sql::new(
                    "SELECT * FROM sales.lines WHERE quote_id=$1 ORDER BY position",
                    "SELECT * FROM sales.lines WHERE quote_id=@P1 ORDER BY position",
                ),
                &[Value::Uuid(id)],
            )
            .await
            .map_err(|e| e.application())?;
        let lines = rows_lines
            .iter()
            .map(|r| {
                Ok(QuoteLineDto {
                    id: r.uuid("id").map_err(|e| e.application())?,
                    description: r.text("description").map_err(|e| e.application())?,
                    quantity: r
                        .decimal("quantity")
                        .map_err(|e| e.application())?
                        .normalize()
                        .to_string(),
                    unit_price: r
                        .decimal("unit_price")
                        .map_err(|e| e.application())?
                        .normalize()
                        .to_string(),
                    total: money(r.decimal("total").map_err(|e| e.application())?),
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;
        let result = RecordResult {
            data: QuoteDto {
                id: q.id,
                number: q.number,
                customer_id: row.uuid("customer_id").map_err(|e| e.application())?,
                customer_code: row.text("customer_code").map_err(|e| e.application())?,
                customer_name: q.customer_name,
                date: q.date,
                currency: q.currency,
                status: q.status,
                lines,
                total: q.total,
                modified_at: row.timestamp("modified_at").map_err(|e| e.application())?,
                submitted_at: row
                    .optional_text("submitted_at")
                    .map_err(|e| e.application())?
                    .map(|t| DateTime::parse_from_rfc3339(&t).map(|d| d.with_timezone(&Utc)))
                    .transpose()
                    .map_err(|_| AppError::Internal)?,
            },
            version: row.text("version").map_err(|e| e.application())?,
            capabilities: None,
        };
        tx.commit().await.map_err(|e| e.application())?;
        Ok(Some(result))
    }
    pub async fn odata(&self, raw: Option<&str>) -> Result<serde_json::Value, AppError> {
        use bqatlas_odata::{DataType as T, Field};
        let fields = vec![
            Field::new("Id", "id", T::Guid),
            Field::new("Number", "number", T::String),
            Field::new("CustomerId", "customer_id", T::Guid),
            Field::new("CustomerName", "customer_name", T::String),
            Field::new("Date", "day", T::Date),
            Field::new("Currency", "currency", T::String),
            Field::new("Status", "status", T::String),
            Field::new("Total", "total", T::Decimal),
        ];
        let query = bqatlas_odata::ReadQuery::parse(raw, &fields)?;
        bqatlas_odata::read(&self.db, "sales.quotes", &fields, query, |row| {
            bqatlas_odata::pascal_projection(
                serde_json::to_value(summary(row)?).map_err(|_| AppError::Internal)?,
            )
        })
        .await
    }
    pub async fn query(&self, request: QueryRequest) -> Result<PageResult<QuoteSummary>, AppError> {
        validate_query(&request, &["number", "date", "status", "customerName"])?;
        if request.filters.as_ref().is_some_and(|f| !f.is_empty()) {
            return Err(invalid(
                "filters",
                "Use search and sorting for the quote list.",
            ));
        }
        let text = search_key(request.search.trim())
            .replace('~', "~~")
            .replace('%', "~%")
            .replace('_', "~_")
            .replace('[', "~[");
        let values = vec![Value::Text(format!("%{text}%"))];
        let pg = "number LIKE $1 ESCAPE '~' OR UPPER(customer_name) LIKE $1 ESCAPE '~'";
        let ms = "number LIKE @P1 ESCAPE '~' OR UPPER(customer_name) LIKE @P1 ESCAPE '~'";
        let rows = self
            .db
            .query(
                Sql::new(
                    &format!("SELECT COUNT(*)::bigint AS total FROM sales.quotes WHERE {pg}"),
                    &format!("SELECT COUNT_BIG(*) AS total FROM sales.quotes WHERE {ms}"),
                ),
                &values,
            )
            .await
            .map_err(|e| e.application())?;
        let total = rows[0].integer("total").map_err(|e| e.application())?;
        let mut order = vec![];
        for sort in request.sort.iter().flatten() {
            let field = match sort.field.as_str() {
                "number" => "number",
                "status" => "status",
                "customerName" => "customer_name",
                _ => "day",
            };
            order.push(format!("{field} {}", sort.direction));
        }
        if order.is_empty() {
            order.push("day DESC".into());
        }
        order.push("id ASC".into());
        let order = order.join(",");
        let pg =
            format!("SELECT * FROM sales.quotes WHERE {pg} ORDER BY {order} LIMIT $2 OFFSET $3");
        let ms = format!(
            "SELECT * FROM sales.quotes WHERE {ms} ORDER BY {order} OFFSET @P3 ROWS FETCH NEXT @P2 ROWS ONLY"
        );
        let values = [
            values[0].clone(),
            Value::Int(request.page_size),
            Value::Int(request.page * request.page_size),
        ];
        let rows = self
            .db
            .query(Sql::new(&pg, &ms), &values)
            .await
            .map_err(|e| e.application())?;
        Ok(PageResult {
            items: rows.iter().map(summary).collect::<Result<_, _>>()?,
            total,
            page: request.page,
            page_size: request.page_size,
        })
    }

    pub async fn save(
        &self,
        id: Option<Uuid>,
        input: QuoteInput,
        expected: Option<&str>,
        actor: &str,
    ) -> Result<RecordResult<QuoteDto>, AppError> {
        self.save_impl(id, input, expected, actor, None).await
    }
    pub async fn seed(&self, id: Uuid, input: QuoteInput) -> Result<bool, AppError> {
        if self.get(id).await?.is_some() {
            return Ok(false);
        }
        self.save_impl(None, input, None, "development-seed", Some(id))
            .await?;
        Ok(true)
    }
    async fn save_impl(
        &self,
        id: Option<Uuid>,
        input: QuoteInput,
        expected: Option<&str>,
        actor: &str,
        create_id: Option<Uuid>,
    ) -> Result<RecordResult<QuoteDto>, AppError> {
        let value = validate(input)?;
        let customer = self
            .customers
            .find(value.customer_id)
            .await?
            .filter(|c| c.active)
            .ok_or_else(|| invalid("customerId", "Choose an active customer."))?;
        let existing = if let Some(id) = id {
            let quote = self.get(id).await?.ok_or(AppError::NotFound)?;
            if quote.version != expected.unwrap_or_default() {
                return Err(AppError::VersionConflict);
            }
            if quote.data.status != "draft" {
                return Err(conflict(
                    "quote_immutable",
                    "A submitted quote cannot be edited.",
                ));
            }
            Some(quote)
        } else {
            None
        };
        let id = id.or(create_id).unwrap_or_else(Uuid::new_v4);
        let version = new_version();
        let now = Utc::now().trunc_subsecs(6);
        let number = existing.as_ref().map_or_else(
            || format!("Q-{}", id.simple().to_string().to_ascii_uppercase()),
            |q| q.data.number.clone(),
        );
        let mut values = vec![
            Value::Uuid(id),
            Value::Text(number),
            Value::Uuid(customer.id),
            Value::Text(customer.code),
            Value::Text(customer.name),
            Value::Date(value.date),
            Value::Text(value.currency),
            Value::Decimal(value.total),
            Value::Text(version.clone()),
            Value::Timestamp(now),
            Value::Text(actor.into()),
        ];
        let mut tx = self.db.begin().await.map_err(|e| e.application())?;
        let changed=if let Some(existing)=&existing{values.push(Value::Text(existing.version.clone()));tx.execute(Sql::new("UPDATE sales.quotes SET number=$2,customer_id=$3,customer_code=$4,customer_name=$5,day=$6,currency=$7,total=$8,version=$9,modified_at=$10,modified_by=$11 WHERE id=$1 AND version=$12 AND status='draft'","UPDATE sales.quotes SET number=@P2,customer_id=@P3,customer_code=@P4,customer_name=@P5,day=@P6,currency=@P7,total=@P8,version=@P9,modified_at=@P10,modified_by=@P11 WHERE id=@P1 AND version=@P12 AND status='draft'"),&values).await}else{tx.execute(Sql::new("INSERT INTO sales.quotes(id,number,customer_id,customer_code,customer_name,day,currency,total,version,modified_at,modified_by,status) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,'draft')","INSERT INTO sales.quotes(id,number,customer_id,customer_code,customer_name,day,currency,total,version,modified_at,modified_by,status) VALUES(@P1,@P2,@P3,@P4,@P5,@P6,@P7,@P8,@P9,@P10,@P11,'draft')"),&values).await}.map_err(|e|e.application())?;
        if changed != 1 {
            return Err(AppError::VersionConflict);
        }
        tx.execute(
            Sql::new(
                "DELETE FROM sales.lines WHERE quote_id=$1",
                "DELETE FROM sales.lines WHERE quote_id=@P1",
            ),
            &[Value::Uuid(id)],
        )
        .await
        .map_err(|e| e.application())?;
        for (position, line) in value.lines.into_iter().enumerate() {
            tx.execute(Sql::new("INSERT INTO sales.lines(quote_id,id,position,description,quantity,unit_price,total) VALUES($1,$2,$3,$4,$5,$6,$7)","INSERT INTO sales.lines(quote_id,id,position,description,quantity,unit_price,total) VALUES(@P1,@P2,@P3,@P4,@P5,@P6,@P7)"),&[Value::Uuid(id),Value::Uuid(line.id),Value::Int(position as i64),Value::Text(line.description),Value::Decimal(line.quantity),Value::Decimal(line.unit_price),Value::Decimal(line.total)]).await.map_err(|e|e.application())?;
        }
        audit(
            &mut tx,
            id,
            actor,
            if existing.is_some() {
                "edited"
            } else {
                "created"
            },
            &version,
            now,
        )
        .await?;
        tx.commit().await.map_err(|e| e.application())?;
        self.get(id).await?.ok_or(AppError::Internal)
    }
    pub async fn delete(&self, id: Uuid, expected: &str, actor: &str) -> Result<(), AppError> {
        let quote = self.get(id).await?.ok_or(AppError::NotFound)?;
        if quote.version != expected {
            return Err(AppError::VersionConflict);
        }
        if quote.data.status != "draft" {
            return Err(conflict(
                "quote_immutable",
                "A submitted quote cannot be deleted.",
            ));
        }
        let mut tx = self.db.begin().await.map_err(|e| e.application())?;
        let changed = tx
            .execute(
                Sql::new(
                    "DELETE FROM sales.quotes WHERE id=$1 AND version=$2 AND status='draft'",
                    "DELETE FROM sales.quotes WHERE id=@P1 AND version=@P2 AND status='draft'",
                ),
                &[Value::Uuid(id), Value::Text(expected.into())],
            )
            .await
            .map_err(|e| e.application())?;
        if changed != 1 {
            return Err(AppError::VersionConflict);
        }
        audit(
            &mut tx,
            id,
            actor,
            "deleted",
            &new_version(),
            Utc::now().trunc_subsecs(6),
        )
        .await?;
        tx.commit().await.map_err(|e| e.application())?;
        Ok(())
    }
    async fn replay(&self, id: Uuid, actor: &str, key: &str) -> Result<bool, AppError> {
        Ok(!self.db.query(Sql::new("SELECT version FROM sales.submissions WHERE quote_id=$1 AND actor=$2 AND key=$3","SELECT version FROM sales.submissions WHERE quote_id=@P1 AND actor=@P2 AND [key]=@P3"),&[Value::Uuid(id),Value::Text(actor.into()),Value::Text(key.into())]).await.map_err(|e|e.application())?.is_empty())
    }
    pub async fn submit(
        &self,
        id: Uuid,
        expected: &str,
        key: &str,
        actor: &str,
    ) -> Result<RecordResult<QuoteDto>, AppError> {
        let key = Uuid::parse_str(key)
            .map_err(|_| invalid("idempotencyKey", "Idempotency-Key must be a UUID."))?
            .simple()
            .to_string();
        if self.replay(id, actor, &key).await? {
            return self.get(id).await?.ok_or(AppError::NotFound);
        }
        let quote = self.get(id).await?.ok_or(AppError::NotFound)?;
        if quote.version != expected {
            if self.replay(id, actor, &key).await? {
                return self.get(id).await?.ok_or(AppError::NotFound);
            }
            return Err(AppError::VersionConflict);
        }
        if quote.data.status != "draft" {
            return Err(conflict(
                "quote_already_submitted",
                "This quote has already been submitted.",
            ));
        }
        if !self
            .customers
            .find(quote.data.customer_id)
            .await?
            .is_some_and(|c| c.active)
        {
            return Err(conflict(
                "customer_unavailable",
                "The customer is no longer active. Choose an active customer before submitting.",
            ));
        }
        let version = new_version();
        let now = Utc::now().trunc_subsecs(6);
        let mut tx = self.db.begin().await.map_err(|e| e.application())?;
        let result:Result<(),AppError>=async{
   let changed=tx.execute(Sql::new("UPDATE sales.quotes SET status='submitted',version=$2,modified_at=$3,modified_by=$4,submitted_at=$5 WHERE id=$1 AND version=$6 AND status='draft'","UPDATE sales.quotes SET status='submitted',version=@P2,modified_at=@P3,modified_by=@P4,submitted_at=@P5 WHERE id=@P1 AND version=@P6 AND status='draft'"),&[Value::Uuid(id),Value::Text(version.clone()),Value::Timestamp(now),Value::Text(actor.into()),Value::Text(now.to_rfc3339()),Value::Text(expected.into())]).await.map_err(|e|e.application())?;
   if changed!=1{return Err(AppError::VersionConflict);}
   tx.execute(Sql::new("INSERT INTO sales.submissions(quote_id,actor,key,version) VALUES($1,$2,$3,$4)","INSERT INTO sales.submissions(quote_id,actor,[key],version) VALUES(@P1,@P2,@P3,@P4)"),&[Value::Uuid(id),Value::Text(actor.into()),Value::Text(key.clone()),Value::Text(version.clone())]).await.map_err(|e|e.application())?;audit(&mut tx,id,actor,"submitted",&version,now).await?;Ok(())
  }.await;
        if let Err(error) = result {
            tx.rollback().await.map_err(|e| e.application())?;
            if self.replay(id, actor, &key).await? {
                return self.get(id).await?.ok_or(AppError::NotFound);
            }
            return Err(error);
        }
        tx.commit().await.map_err(|e| e.application())?;
        self.get(id).await?.ok_or(AppError::NotFound)
    }
}
async fn audit(
    tx: &mut Transaction,
    id: Uuid,
    actor: &str,
    operation: &str,
    version: &str,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    tx.execute(Sql::new("INSERT INTO sales.operations(id,quote_id,operation,actor,occurred_at,version) VALUES($1,$2,$3,$4,$5,$6)","INSERT INTO sales.operations(id,quote_id,operation,actor,occurred_at,version) VALUES(@P1,@P2,@P3,@P4,@P5,@P6)"),&[Value::Uuid(Uuid::new_v4()),Value::Uuid(id),Value::Text(operation.into()),Value::Text(actor.into()),Value::Timestamp(now),Value::Text(version.into())]).await.map_err(|e|e.application())?;
    Ok(())
}
fn quote_record(
    mut result: RecordResult<QuoteDto>,
    actor: &bqatlas_core::Actor,
    created: Option<String>,
) -> ApiResult<Response> {
    let draft = result.data.status == "draft";
    result.capabilities = Some(bqatlas_contracts::RecordCapabilities {
        edit: draft
            && actor.permissions.contains(PERMISSIONS[1])
            && actor.permissions.contains(CUSTOMER_LOOKUP),
        delete: draft && actor.permissions.contains(PERMISSIONS[2]),
        commands: if draft && actor.permissions.contains(PERMISSIONS[3]) {
            vec!["submit".into()]
        } else {
            vec![]
        },
    });
    record(result, created)
}
pub fn router() -> Router<QuoteService> {
    Router::new()
        .route("/odata/Quotes", get(odata_read))
        .route("/api/v1/sales/quotes/query", post(query))
        .route("/api/v1/sales/quotes", post(create))
        .route(
            "/api/v1/sales/quotes/{id}",
            get(read).put(update).delete(delete),
        )
        .route("/api/v1/sales/quotes/{id}/submit", post(submit))
}
async fn query(
    State(service): State<QuoteService>,
    CurrentActor(actor): CurrentActor,
    ApiJson(request): ApiJson<QueryRequest>,
) -> ApiResult<Json<PageResult<QuoteSummary>>> {
    actor.require(PERMISSIONS[0])?;
    Ok(Json(service.query(request).await?))
}
async fn read(
    State(service): State<QuoteService>,
    CurrentActor(actor): CurrentActor,
    Path(id): Path<Uuid>,
) -> ApiResult<Response> {
    actor.require(PERMISSIONS[0])?;
    quote_record(
        service.get(id).await?.ok_or(AppError::NotFound)?,
        &actor,
        None,
    )
}
async fn create(
    State(service): State<QuoteService>,
    CurrentActor(actor): CurrentActor,
    ApiJson(input): ApiJson<QuoteInput>,
) -> ApiResult<Response> {
    actor.require(PERMISSIONS[0])?;
    actor.require(PERMISSIONS[1])?;
    actor.require(CUSTOMER_LOOKUP)?;
    let result = service.save(None, input, None, &actor.id).await?;
    let path = format!("/api/v1/sales/quotes/{}", result.data.id);
    quote_record(result, &actor, Some(path))
}
async fn update(
    State(service): State<QuoteService>,
    CurrentActor(actor): CurrentActor,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    ApiJson(input): ApiJson<QuoteInput>,
) -> ApiResult<Response> {
    actor.require(PERMISSIONS[0])?;
    actor.require(PERMISSIONS[1])?;
    actor.require(CUSTOMER_LOOKUP)?;
    let version = expected_version(headers.get("If-Match").and_then(|v| v.to_str().ok()))?;
    quote_record(
        service
            .save(Some(id), input, Some(version), &actor.id)
            .await?,
        &actor,
        None,
    )
}
async fn delete(
    State(service): State<QuoteService>,
    CurrentActor(actor): CurrentActor,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    actor.require(PERMISSIONS[0])?;
    actor.require(PERMISSIONS[2])?;
    service
        .delete(
            id,
            expected_version(headers.get("If-Match").and_then(|v| v.to_str().ok()))?,
            &actor.id,
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn submit(
    State(service): State<QuoteService>,
    CurrentActor(actor): CurrentActor,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    actor.require(PERMISSIONS[0])?;
    actor.require(PERMISSIONS[3])?;
    quote_record(
        service
            .submit(
                id,
                expected_version(headers.get("If-Match").and_then(|v| v.to_str().ok()))?,
                headers
                    .get("Idempotency-Key")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or_default(),
                &actor.id,
            )
            .await?,
        &actor,
        None,
    )
}

async fn odata_read(
    State(service): State<QuoteService>,
    CurrentActor(actor): CurrentActor,
    axum::extract::RawQuery(raw): axum::extract::RawQuery,
) -> ApiResult<axum::Json<serde_json::Value>> {
    actor.require(PERMISSIONS[0])?;
    Ok(axum::Json(service.odata(raw.as_deref()).await?))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_decimal_fixtures_match_dotnet_and_typescript() {
        let fixtures: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../contracts/v1/fixtures/quote-totals.json"
        ))
        .expect("fixtures");
        for fixture in fixtures.as_array().expect("fixtures") {
            let lines = fixture["lines"]
                .as_array()
                .expect("lines")
                .iter()
                .map(|l| QuoteLineInput {
                    id: Uuid::new_v4(),
                    description: "Line".into(),
                    quantity: l["quantity"].as_str().expect("quantity").into(),
                    unit_price: l["unitPrice"].as_str().expect("price").into(),
                })
                .collect();
            let result = validate(QuoteInput {
                customer_id: Uuid::new_v4(),
                date: NaiveDate::from_ymd_opt(2026, 9, 30).expect("date"),
                currency: "USD".into(),
                lines,
            })
            .expect("valid quote");
            assert_eq!(
                money(result.total),
                fixture["total"].as_str().expect("total")
            );
            for (line, expected) in result
                .lines
                .iter()
                .zip(fixture["lineTotals"].as_array().expect("totals"))
            {
                assert_eq!(money(line.total), expected.as_str().expect("line total"));
            }
        }
    }
    #[test]
    fn quote_validation_reports_line_paths_and_duplicate_ids() {
        let id = Uuid::new_v4();
        let input = QuoteInput {
            customer_id: Uuid::new_v4(),
            date: NaiveDate::from_ymd_opt(2026, 9, 30).expect("date"),
            currency: "USD".into(),
            lines: vec![
                QuoteLineInput {
                    id,
                    description: "Line".into(),
                    quantity: "-1".into(),
                    unit_price: "1e3".into(),
                },
                QuoteLineInput {
                    id,
                    description: "Line".into(),
                    quantity: "1".into(),
                    unit_price: "1".into(),
                },
            ],
        };
        let Err(AppError::Validation(errors)) = validate(input) else {
            panic!("validation")
        };
        assert!(errors.contains_key("lines[0].quantity"));
        assert!(errors.contains_key("lines[0].unitPrice"));
        assert!(errors.contains_key("lines[1].id"));
    }
}
