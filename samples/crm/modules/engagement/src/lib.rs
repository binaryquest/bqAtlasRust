//! CRM engagement module with explicit write inputs and owned database tables.
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
    AppError, ModuleDefinition, decimal_text, expected_version, invalid, new_version, search_key,
    validate_query,
};
use bqatlas_db::{Database, DbRow, Migration, Sql, Value};
use bqatlas_http::ApiPath as Path;
use bqatlas_http::{ApiJson, ApiResult, CurrentActor, record};
use bqatlas_sample_crm_contracts::{CUSTOMER_LOOKUP, CustomerDirectory};
use chrono::{Datelike, NaiveDate, SubsecRound, Utc};
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::{Value as JsonValue, json};
use std::sync::Arc;
use uuid::Uuid;
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Products,
    Opportunities,
    Activities,
}
impl Kind {
    pub fn parse(kind: &str) -> Result<Self, AppError> {
        match kind {
            "products" => Ok(Self::Products),
            "opportunities" => Ok(Self::Opportunities),
            "activities" => Ok(Self::Activities),
            _ => Err(AppError::NotFound),
        }
    }
    fn table(self) -> &'static str {
        match self {
            Self::Products => "products",
            Self::Opportunities => "opportunities",
            Self::Activities => "activities",
        }
    }
    fn permission(self, operation: &str) -> String {
        format!("engagement.{}.{operation}", self.table())
    }
    fn columns(self) -> Vec<(&'static str, &'static str, &'static str)> {
        match self {
            Self::Products => vec![
                ("name", "name", "string"),
                ("code", "code", "string"),
                ("category", "category", "string"),
                ("unitPrice", "unit_price", "decimal4"),
                ("currency", "currency", "string"),
                ("active", "active", "boolean"),
            ],
            Self::Opportunities => vec![
                ("name", "name", "string"),
                ("customerId", "customer_id", "uuid"),
                ("customerName", "customer_name", "string"),
                ("stage", "stage", "string"),
                ("amount", "amount", "decimal2"),
                ("currency", "currency", "string"),
                ("expectedClose", "expected_close", "date"),
                ("owner", "owner", "string"),
                ("notes", "notes", "string"),
            ],
            Self::Activities => vec![
                ("name", "name", "string"),
                ("customerId", "customer_id", "uuid"),
                ("customerName", "customer_name", "string"),
                ("dueDate", "due_date", "date"),
                ("kind", "kind", "string"),
                ("status", "status", "string"),
                ("owner", "owner", "string"),
                ("notes", "notes", "string"),
            ],
        }
    }
}
pub fn definition() -> ModuleDefinition {
    ModuleDefinition {
        id: "engagement".into(),
        dependencies: vec!["crm".into()],
    }
}
pub fn migration() -> Migration {
    Migration {
        module: "engagement",
        version: 1,
        sql: Sql::new(
            include_str!("../migrations/postgresql/0001.sql"),
            include_str!("../migrations/sqlserver/0001.sql"),
        ),
    }
}
pub fn permissions() -> Vec<String> {
    [Kind::Products, Kind::Opportunities, Kind::Activities]
        .into_iter()
        .flat_map(|kind| ["read", "write", "delete"].map(|op| kind.permission(op)))
        .collect()
}
fn field(
    name: &str,
    label: &str,
    kind: &str,
    required: bool,
    max: Option<usize>,
) -> FieldDescriptor {
    FieldDescriptor::new(name, label, kind, required, max)
}
fn choice(name: &str, label: &str, options: &[&str]) -> FieldDescriptor {
    let mut f = field(name, label, "enum", true, None);
    f.options = Some(options.iter().map(|s| (*s).into()).collect());
    f
}
fn decimal_field(name: &str, label: &str, scale: u32) -> FieldDescriptor {
    let mut f = field(name, label, "decimal", true, None);
    f.scale = Some(scale);
    f.maximum = Some("1000000000".into());
    f
}
pub fn descriptors() -> Vec<ResourceDescriptor> {
    [Kind::Products, Kind::Opportunities, Kind::Activities]
        .into_iter()
        .map(|kind| {
            let fields = match kind {
                Kind::Products => vec![
                    field("code", "SKU", "string", true, Some(32)),
                    field("name", "Product", "string", true, Some(200)),
                    choice("category", "Category", &["Stock", "Service"]),
                    decimal_field("unitPrice", "Unit price", 4),
                    choice("currency", "Currency", &["USD", "EUR", "BDT"]),
                    field("active", "Active", "boolean", false, None),
                ],
                Kind::Opportunities => vec![
                    field("name", "Opportunity", "string", true, Some(200)),
                    field("customerId", "Customer", "string", true, None),
                    choice(
                        "stage",
                        "Stage",
                        &["Lead", "Qualified", "Proposal", "Won", "Lost"],
                    ),
                    decimal_field("amount", "Expected value", 2),
                    choice("currency", "Currency", &["USD", "EUR", "BDT"]),
                    field("expectedClose", "Expected close", "date", true, None),
                    field("owner", "Owner", "string", true, Some(100)),
                    field("notes", "Notes", "string", false, Some(2000)),
                ],
                Kind::Activities => vec![
                    field("name", "Subject", "string", true, Some(200)),
                    field("customerId", "Customer", "string", true, None),
                    choice("kind", "Type", &["Call", "Email", "Meeting", "Task"]),
                    field("dueDate", "Due date", "date", true, None),
                    choice("status", "Status", &["Planned", "Done", "Cancelled"]),
                    field("owner", "Owner", "string", true, Some(100)),
                    field("notes", "Notes", "string", false, Some(2000)),
                ],
            };
            ResourceDescriptor {
                id: format!("engagement.{}", kind.table()),
                title: match kind {
                    Kind::Products => "Products",
                    Kind::Opportunities => "Pipeline",
                    Kind::Activities => "Activities",
                }
                .into(),
                endpoint: format!("/api/v1/engagement/{}", kind.table()),
                read_permission: kind.permission("read"),
                fields,
                key_field: "id".into(),
                o_data_endpoint: Some(format!(
                    "/odata/{}",
                    match kind {
                        Kind::Products => "Products",
                        Kind::Opportunities => "Opportunities",
                        Kind::Activities => "Activities",
                    }
                )),
            }
        })
        .collect()
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProductInput {
    name: String,
    code: String,
    category: String,
    unit_price: String,
    currency: String,
    active: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpportunityInput {
    name: String,
    customer_id: Uuid,
    stage: String,
    amount: String,
    currency: String,
    expected_close: NaiveDate,
    owner: String,
    #[serde(default)]
    notes: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActivityInput {
    name: String,
    customer_id: Uuid,
    due_date: NaiveDate,
    kind: String,
    status: String,
    owner: String,
    #[serde(default)]
    notes: String,
}
fn text(value: String, field: &str, max: usize, required: bool) -> Result<String, AppError> {
    let value = value.trim().to_owned();
    if (required && value.is_empty()) || value.encode_utf16().count() > max {
        Err(invalid(
            field,
            &format!("Use {}–{max} characters.", if required { 1 } else { 0 }),
        ))
    } else {
        Ok(value)
    }
}
fn chosen(value: String, field: &str, options: &[&str]) -> Result<String, AppError> {
    if options.contains(&value.as_str()) {
        Ok(value)
    } else {
        Err(invalid(field, "Choose one of the listed values."))
    }
}
fn money(value: &str, field: &str, scale: u32) -> Result<Decimal, AppError> {
    let (whole, fraction) = value
        .split_once('.')
        .map_or((value, None), |(w, f)| (w, Some(f)));
    let whole = whole.trim_start_matches('0');
    let normalized = format!(
        "{}{}",
        if whole.is_empty() { "0" } else { whole },
        fraction.map_or(String::new(), |f| format!(".{f}"))
    );
    if value.is_empty() || value.starts_with('.') {
        return Err(invalid(field, "Enter a valid amount."));
    }
    decimal_text(&normalized, scale, Decimal::from(1_000_000_000)).ok_or_else(|| {
        invalid(
            field,
            &format!("Enter an amount from 0 to 1000000000 with up to {scale} decimal places."),
        )
    })
}
fn date(value: NaiveDate, field: &str) -> Result<NaiveDate, AppError> {
    if value.year() < 1900 {
        Err(invalid(field, "Choose a date from 1900 onwards."))
    } else {
        Ok(value)
    }
}
#[derive(Clone)]
pub struct EngagementService {
    db: Database,
    customers: Arc<dyn CustomerDirectory>,
}
impl EngagementService {
    pub fn new(db: Database, customers: Arc<dyn CustomerDirectory>) -> Self {
        Self { db, customers }
    }
    async fn inputs(&self, kind: Kind, input: JsonValue) -> Result<Vec<Value>, AppError> {
        match kind {
            Kind::Products => {
                let input: ProductInput = serde_json::from_value(input)
                    .map_err(|_| AppError::BadRequest("Invalid product fields.".into()))?;
                let name = text(input.name, "name", 200, true)?;
                let code = text(input.code, "code", 32, true)?.to_ascii_uppercase();
                if !code.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-') {
                    return Err(invalid("code", "Use letters, digits and hyphens."));
                }
                Ok(vec![
                    Value::Text(name),
                    Value::Text(code),
                    Value::Text(chosen(input.category, "category", &["Stock", "Service"])?),
                    Value::Decimal(money(&input.unit_price, "unitPrice", 4)?),
                    Value::Text(chosen(input.currency, "currency", &["USD", "EUR", "BDT"])?),
                    Value::Bool(input.active),
                ])
            }
            Kind::Opportunities => {
                let input: OpportunityInput = serde_json::from_value(input)
                    .map_err(|_| AppError::BadRequest("Invalid opportunity fields.".into()))?;
                let customer = self
                    .customers
                    .find(input.customer_id)
                    .await?
                    .filter(|c| c.active)
                    .ok_or_else(|| invalid("customerId", "Choose an active customer."))?;
                Ok(vec![
                    Value::Text(text(input.name, "name", 200, true)?),
                    Value::Uuid(customer.id),
                    Value::Text(customer.name),
                    Value::Text(chosen(
                        input.stage,
                        "stage",
                        &["Lead", "Qualified", "Proposal", "Won", "Lost"],
                    )?),
                    Value::Decimal(money(&input.amount, "amount", 2)?),
                    Value::Text(chosen(input.currency, "currency", &["USD", "EUR", "BDT"])?),
                    Value::Date(date(input.expected_close, "expectedClose")?),
                    Value::Text(text(input.owner, "owner", 100, true)?),
                    Value::Text(text(input.notes, "notes", 2000, false)?),
                ])
            }
            Kind::Activities => {
                let input: ActivityInput = serde_json::from_value(input)
                    .map_err(|_| AppError::BadRequest("Invalid activity fields.".into()))?;
                let customer = self
                    .customers
                    .find(input.customer_id)
                    .await?
                    .filter(|c| c.active)
                    .ok_or_else(|| invalid("customerId", "Choose an active customer."))?;
                Ok(vec![
                    Value::Text(text(input.name, "name", 200, true)?),
                    Value::Uuid(customer.id),
                    Value::Text(customer.name),
                    Value::Date(date(input.due_date, "dueDate")?),
                    Value::Text(chosen(
                        input.kind,
                        "kind",
                        &["Call", "Email", "Meeting", "Task"],
                    )?),
                    Value::Text(chosen(
                        input.status,
                        "status",
                        &["Planned", "Done", "Cancelled"],
                    )?),
                    Value::Text(text(input.owner, "owner", 100, true)?),
                    Value::Text(text(input.notes, "notes", 2000, false)?),
                ])
            }
        }
    }
    pub async fn get(
        &self,
        kind: Kind,
        id: Uuid,
    ) -> Result<Option<RecordResult<JsonValue>>, AppError> {
        let table = kind.table();
        let rows = self
            .db
            .query(
                Sql::new(
                    &format!("SELECT * FROM engagement.{table} WHERE id=$1"),
                    &format!("SELECT * FROM engagement.{table} WHERE id=@P1"),
                ),
                &[Value::Uuid(id)],
            )
            .await
            .map_err(|e| e.application())?;
        rows.first()
            .map(|row| {
                Ok(RecordResult {
                    data: project(kind, row)?,
                    version: row.text("version").map_err(|e| e.application())?,
                    capabilities: None,
                })
            })
            .transpose()
    }

    pub async fn save(
        &self,
        kind: Kind,
        id: Option<Uuid>,
        input: JsonValue,
        expected: Option<&str>,
        actor: &str,
    ) -> Result<RecordResult<JsonValue>, AppError> {
        self.save_impl(kind, id, input, expected, actor, None).await
    }
    pub async fn seed(&self, kind: Kind, id: Uuid, input: JsonValue) -> Result<bool, AppError> {
        if self.get(kind, id).await?.is_some() {
            return Ok(false);
        }
        self.save_impl(kind, None, input, None, "development-seed", Some(id))
            .await?;
        Ok(true)
    }
    async fn save_impl(
        &self,
        kind: Kind,
        id: Option<Uuid>,
        input: JsonValue,
        expected: Option<&str>,
        actor: &str,
        create_id: Option<Uuid>,
    ) -> Result<RecordResult<JsonValue>, AppError> {
        let existing = if let Some(id) = id {
            let row = self.get(kind, id).await?.ok_or(AppError::NotFound)?;
            if row.version != expected.unwrap_or_default() {
                return Err(AppError::VersionConflict);
            }
            Some(row)
        } else {
            None
        };
        let fields = self.inputs(kind, input).await?;
        let columns = kind.columns();
        let id = id.or(create_id).unwrap_or_else(Uuid::new_v4);
        let table = kind.table();
        if kind == Kind::Products {
            let duplicate = self
                .db
                .query(
                    Sql::new(
                        "SELECT id FROM engagement.products WHERE code=$1 AND id<>$2",
                        "SELECT id FROM engagement.products WHERE code=@P1 AND id<>@P2",
                    ),
                    &[fields[1].clone(), Value::Uuid(id)],
                )
                .await
                .map_err(|e| e.application())?;
            if !duplicate.is_empty() {
                return Err(invalid("code", "This SKU already exists."));
            }
        }
        let search = search_key(
            &fields
                .iter()
                .zip(&columns)
                .filter_map(|(value, (name, _, _))| {
                    if matches!(*name, "name" | "code" | "customerName" | "owner") {
                        if let Value::Text(value) = value {
                            Some(value.as_str())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join(" "),
        );
        let mut values = vec![Value::Uuid(id)];
        values.extend(fields);
        values.extend([
            Value::Text(search),
            Value::Text(new_version()),
            Value::Timestamp(Utc::now().trunc_subsecs(6)),
            Value::Text(actor.into()),
        ]);
        let mut names = vec!["id"];
        names.extend(columns.iter().map(|(_, column, _)| *column));
        names.extend(["search_name", "version", "modified_at", "modified_by"]);
        let (pg, ms) = if let Some(existing) = existing {
            let updates_pg = names
                .iter()
                .enumerate()
                .skip(1)
                .map(|(i, n)| format!("{n}=${}", i + 1))
                .collect::<Vec<_>>()
                .join(",");
            let updates_ms = names
                .iter()
                .enumerate()
                .skip(1)
                .map(|(i, n)| format!("{n}=@P{}", i + 1))
                .collect::<Vec<_>>()
                .join(",");
            values.push(Value::Text(existing.version));
            let n = values.len();
            (
                format!("UPDATE engagement.{table} SET {updates_pg} WHERE id=$1 AND version=${n}"),
                format!(
                    "UPDATE engagement.{table} SET {updates_ms} WHERE id=@P1 AND version=@P{n}"
                ),
            )
        } else {
            let names = names.join(",");
            let placeholders_pg = (1..=values.len())
                .map(|i| format!("${i}"))
                .collect::<Vec<_>>()
                .join(",");
            let placeholders_ms = (1..=values.len())
                .map(|i| format!("@P{i}"))
                .collect::<Vec<_>>()
                .join(",");
            (
                format!("INSERT INTO engagement.{table}({names}) VALUES({placeholders_pg})"),
                format!("INSERT INTO engagement.{table}({names}) VALUES({placeholders_ms})"),
            )
        };
        if self
            .db
            .execute(Sql::new(&pg, &ms), &values)
            .await
            .map_err(|e| e.application())?
            != 1
        {
            return Err(AppError::VersionConflict);
        }
        self.get(kind, id).await?.ok_or(AppError::Internal)
    }
    pub async fn odata(&self, kind: Kind, raw: Option<&str>) -> Result<JsonValue, AppError> {
        use bqatlas_odata::{DataType as T, Field};
        let mut fields = vec![
            Field::new("Id", "id", T::Guid),
            Field::new("ModifiedAt", "modified_at", T::Timestamp),
        ];
        for (name, column, ty) in kind.columns() {
            let mut chars = name.chars();
            let property = format!(
                "{}{}",
                chars.next().ok_or(AppError::Internal)?.to_ascii_uppercase(),
                chars.as_str()
            );
            let ty = match ty {
                "boolean" => T::Boolean,
                "uuid" => T::Guid,
                "date" => T::Date,
                _ if ty.starts_with("decimal") => T::Decimal,
                _ => T::String,
            };
            fields.push(Field::new(&property, column, ty));
        }
        let query = bqatlas_odata::ReadQuery::parse(raw, &fields)?;
        bqatlas_odata::read(
            &self.db,
            &format!("engagement.{}", kind.table()),
            &fields,
            query,
            |row| bqatlas_odata::pascal_projection(project(kind, row)?),
        )
        .await
    }
    pub async fn query(
        &self,
        kind: Kind,
        request: QueryRequest,
    ) -> Result<PageResult<JsonValue>, AppError> {
        let columns = kind.columns();
        let fields = columns
            .iter()
            .filter(|(name, _, ty)| !ty.starts_with("decimal") && *name != "customerName")
            .map(|(name, _, _)| *name)
            .collect::<Vec<_>>();
        validate_query(&request, &fields)?;
        let table = kind.table();
        let mut values = vec![];
        let mut pg = vec!["1=1".into()];
        let mut ms = pg.clone();
        if !request.search.trim().is_empty() {
            let text = search_key(request.search.trim())
                .replace('~', "~~")
                .replace('%', "~%")
                .replace('_', "~_")
                .replace('[', "~[");
            values.push(Value::Text(format!("%{text}%")));
            pg.push("search_name LIKE $1 ESCAPE '~'".into());
            ms.push("search_name LIKE @P1 ESCAPE '~'".into());
        }
        for filter in request.filters.iter().flatten() {
            if kind == Kind::Products || filter.field != "customerId" || filter.operator != "eq" {
                return Err(invalid("filters", "Only customerId eq is supported."));
            }
            let id = Uuid::parse_str(&filter.value)
                .map_err(|_| invalid("filters", "Choose a valid customer ID."))?;
            values.push(Value::Uuid(id));
            let n = values.len();
            pg.push(format!("customer_id=${n}"));
            ms.push(format!("customer_id=@P{n}"));
        }
        let pg = pg.join(" AND ");
        let ms = ms.join(" AND ");
        let rows = self
            .db
            .query(
                Sql::new(
                    &format!("SELECT COUNT(*)::bigint AS total FROM engagement.{table} WHERE {pg}"),
                    &format!("SELECT COUNT_BIG(*) AS total FROM engagement.{table} WHERE {ms}"),
                ),
                &values,
            )
            .await
            .map_err(|e| e.application())?;
        let total = rows[0].integer("total").map_err(|e| e.application())?;
        let mut order = vec![];
        for sort in request.sort.iter().flatten() {
            let column = columns
                .iter()
                .find(|(name, _, _)| *name == sort.field)
                .ok_or(AppError::Internal)?
                .1;
            order.push(format!("{column} {}", sort.direction));
        }
        if order.is_empty() {
            order.push("name ASC".into());
        }
        order.push("id ASC".into());
        let order = order.join(",");
        let limit = values.len() + 1;
        let offset = limit + 1;
        values.push(Value::Int(request.page_size));
        values.push(Value::Int(request.page * request.page_size));
        let rows=self.db.query(Sql::new(&format!("SELECT * FROM engagement.{table} WHERE {pg} ORDER BY {order} LIMIT ${limit} OFFSET ${offset}"),&format!("SELECT * FROM engagement.{table} WHERE {ms} ORDER BY {order} OFFSET @P{offset} ROWS FETCH NEXT @P{limit} ROWS ONLY")),&values).await.map_err(|e|e.application())?;
        Ok(PageResult {
            items: rows
                .iter()
                .map(|r| project(kind, r))
                .collect::<Result<_, _>>()?,
            total,
            page: request.page,
            page_size: request.page_size,
        })
    }
    pub async fn delete(&self, kind: Kind, id: Uuid, expected: &str) -> Result<(), AppError> {
        let record = self.get(kind, id).await?.ok_or(AppError::NotFound)?;
        if record.version != expected {
            return Err(AppError::VersionConflict);
        }
        let table = kind.table();
        if self
            .db
            .execute(
                Sql::new(
                    &format!("DELETE FROM engagement.{table} WHERE id=$1 AND version=$2"),
                    &format!("DELETE FROM engagement.{table} WHERE id=@P1 AND version=@P2"),
                ),
                &[Value::Uuid(id), Value::Text(expected.into())],
            )
            .await
            .map_err(|e| e.application())?
            != 1
        {
            return Err(AppError::VersionConflict);
        }
        Ok(())
    }
}
fn project(kind: Kind, row: &DbRow) -> Result<JsonValue, AppError> {
    let mut dto = serde_json::Map::new();
    dto.insert(
        "id".into(),
        json!(row.uuid("id").map_err(|e| e.application())?),
    );
    dto.insert(
        "modifiedAt".into(),
        json!(row.timestamp("modified_at").map_err(|e| e.application())?),
    );
    for (name, column, ty) in kind.columns() {
        let value = match ty {
            "uuid" => json!(row.uuid(column).map_err(|e| e.application())?),
            "date" => json!(row.date(column).map_err(|e| e.application())?),
            "decimal4" => json!(format!(
                "{:.4}",
                row.decimal(column).map_err(|e| e.application())?
            )),
            "decimal2" => json!(format!(
                "{:.2}",
                row.decimal(column).map_err(|e| e.application())?
            )),
            "boolean" => json!(row.boolean(column).map_err(|e| e.application())?),
            _ => json!(row.text(column).map_err(|e| e.application())?),
        };
        dto.insert(name.into(), value);
    }
    Ok(JsonValue::Object(dto))
}
pub fn router() -> Router<EngagementService> {
    Router::new()
        .route("/odata/Products", get(odata_products))
        .route("/odata/Opportunities", get(odata_opportunities))
        .route("/odata/Activities", get(odata_activities))
        .route("/api/v1/engagement/{kind}/query", post(query))
        .route("/api/v1/engagement/{kind}/lookup", post(lookup))
        .route("/api/v1/engagement/{kind}/lookup/{id}", get(resolve))
        .route("/api/v1/engagement/{kind}", post(create))
        .route(
            "/api/v1/engagement/{kind}/{id}",
            get(read).put(update).delete(delete),
        )
}
async fn query(
    State(service): State<EngagementService>,
    CurrentActor(actor): CurrentActor,
    Path(kind): Path<String>,
    ApiJson(request): ApiJson<QueryRequest>,
) -> ApiResult<Json<PageResult<JsonValue>>> {
    let kind = Kind::parse(&kind)?;
    actor.require(&kind.permission("read"))?;
    Ok(Json(service.query(kind, request).await?))
}
async fn lookup(
    State(service): State<EngagementService>,
    CurrentActor(actor): CurrentActor,
    Path(kind): Path<String>,
    ApiJson(request): ApiJson<QueryRequest>,
) -> ApiResult<Json<PageResult<JsonValue>>> {
    let kind = Kind::parse(&kind)?;
    if kind != Kind::Products {
        return Err(AppError::NotFound.into());
    }
    actor.require(&kind.permission("read"))?;
    Ok(Json(service.query(kind, request).await?))
}
async fn resolve(
    State(service): State<EngagementService>,
    CurrentActor(actor): CurrentActor,
    Path((kind, id)): Path<(String, Uuid)>,
) -> ApiResult<Json<Option<JsonValue>>> {
    let kind = Kind::parse(&kind)?;
    if kind != Kind::Products {
        return Err(AppError::NotFound.into());
    }
    actor.require(&kind.permission("read"))?;
    Ok(Json(service.get(kind, id).await?.map(|r| r.data)))
}
async fn read(
    State(service): State<EngagementService>,
    CurrentActor(actor): CurrentActor,
    Path((kind, id)): Path<(String, Uuid)>,
) -> ApiResult<Response> {
    let kind = Kind::parse(&kind)?;
    actor.require(&kind.permission("read"))?;
    record(
        service.get(kind, id).await?.ok_or(AppError::NotFound)?,
        None,
    )
}
async fn create(
    State(service): State<EngagementService>,
    CurrentActor(actor): CurrentActor,
    Path(kind): Path<String>,
    ApiJson(input): ApiJson<JsonValue>,
) -> ApiResult<Response> {
    let kind = Kind::parse(&kind)?;
    actor.require(&kind.permission("read"))?;
    actor.require(&kind.permission("write"))?;
    if kind != Kind::Products {
        actor.require(CUSTOMER_LOOKUP)?;
    }
    let result = service.save(kind, None, input, None, &actor.id).await?;
    let path = format!(
        "/api/v1/engagement/{}/{}",
        kind.table(),
        result.data["id"].as_str().ok_or(AppError::Internal)?
    );
    record(result, Some(path))
}
async fn update(
    State(service): State<EngagementService>,
    CurrentActor(actor): CurrentActor,
    Path((kind, id)): Path<(String, Uuid)>,
    headers: HeaderMap,
    ApiJson(input): ApiJson<JsonValue>,
) -> ApiResult<Response> {
    let kind = Kind::parse(&kind)?;
    actor.require(&kind.permission("read"))?;
    actor.require(&kind.permission("write"))?;
    if kind != Kind::Products {
        actor.require(CUSTOMER_LOOKUP)?;
    }
    record(
        service
            .save(
                kind,
                Some(id),
                input,
                Some(expected_version(
                    headers.get("If-Match").and_then(|h| h.to_str().ok()),
                )?),
                &actor.id,
            )
            .await?,
        None,
    )
}
async fn delete(
    State(service): State<EngagementService>,
    CurrentActor(actor): CurrentActor,
    Path((kind, id)): Path<(String, Uuid)>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    let kind = Kind::parse(&kind)?;
    actor.require(&kind.permission("read"))?;
    actor.require(&kind.permission("delete"))?;
    service
        .delete(
            kind,
            id,
            expected_version(headers.get("If-Match").and_then(|h| h.to_str().ok()))?,
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn odata_products(
    State(service): State<EngagementService>,
    CurrentActor(actor): CurrentActor,
    axum::extract::RawQuery(raw): axum::extract::RawQuery,
) -> ApiResult<Json<JsonValue>> {
    actor.require(&Kind::Products.permission("read"))?;
    Ok(Json(service.odata(Kind::Products, raw.as_deref()).await?))
}

async fn odata_opportunities(
    State(service): State<EngagementService>,
    CurrentActor(actor): CurrentActor,
    axum::extract::RawQuery(raw): axum::extract::RawQuery,
) -> ApiResult<Json<JsonValue>> {
    actor.require(&Kind::Opportunities.permission("read"))?;
    Ok(Json(
        service.odata(Kind::Opportunities, raw.as_deref()).await?,
    ))
}

async fn odata_activities(
    State(service): State<EngagementService>,
    CurrentActor(actor): CurrentActor,
    axum::extract::RawQuery(raw): axum::extract::RawQuery,
) -> ApiResult<Json<JsonValue>> {
    actor.require(&Kind::Activities.permission("read"))?;
    Ok(Json(service.odata(Kind::Activities, raw.as_deref()).await?))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decimals_preserve_scale_and_reject_exponents() {
        assert_eq!(
            format!("{:.4}", money("0012.1255", "unitPrice", 4).expect("money")),
            "12.1255"
        );
        for value in ["1e3", "-1", "1.00001", "1000000001"] {
            assert!(money(value, "unitPrice", 4).is_err());
        }
    }
    #[test]
    fn public_metadata_excludes_snapshots_and_server_fields() {
        for resource in descriptors() {
            assert!(
                !resource
                    .fields
                    .iter()
                    .any(|f| matches!(f.name.as_str(), "version" | "modifiedBy" | "customerName"))
            );
        }
        assert!(Kind::parse("products; DROP TABLE users").is_err());
    }
}
