//! CRM customer module. Its persistence is private; consumers use CustomerDirectory.
use async_trait::async_trait;
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
    AppError, ModuleDefinition, ValidationErrors, expected_version, invalid, new_version,
    search_key, validate_query,
};
use bqatlas_db::{Database, DbRow, Migration, Sql, Value};
use bqatlas_http::ApiPath as Path;
use bqatlas_http::{ApiJson, ApiResult, CurrentActor, record};
use bqatlas_sample_crm_contracts::{CUSTOMER_LOOKUP, CustomerDirectory, CustomerSummary};
use chrono::{DateTime, SubsecRound, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
pub const PERMISSIONS: [&str; 4] = [
    "crm.customers.read",
    "crm.customers.write",
    "crm.customers.delete",
    CUSTOMER_LOOKUP,
];
pub fn definition() -> ModuleDefinition {
    ModuleDefinition {
        id: "crm".into(),
        dependencies: vec![],
    }
}
pub fn descriptor() -> ResourceDescriptor {
    ResourceDescriptor {
        id: "crm.customers".into(),
        title: "Customers".into(),
        endpoint: "/api/v1/crm/customers".into(),
        read_permission: PERMISSIONS[0].into(),
        key_field: "id".into(),
        o_data_endpoint: Some("/odata/Customers".into()),
        fields: vec![
            FieldDescriptor::new("code", "Code", "string", true, Some(32)),
            FieldDescriptor::new("name", "Name", "string", true, Some(200)),
            FieldDescriptor::new("email", "Email", "email", false, Some(254)),
            FieldDescriptor::new("active", "Active", "boolean", false, None),
        ],
    }
}
pub fn migration() -> Migration {
    Migration {
        module: "crm",
        version: 1,
        sql: Sql::new(
            include_str!("../migrations/postgresql/0001.sql"),
            include_str!("../migrations/sqlserver/0001.sql"),
        ),
    }
}
#[derive(Debug, Clone, Deserialize)]
pub struct CustomerInput {
    pub code: String,
    pub name: String,
    pub email: Option<String>,
    #[serde(default = "default_active")]
    pub active: bool,
}
fn default_active() -> bool {
    true
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomerDto {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub email: Option<String>,
    pub active: bool,
    pub modified_at: DateTime<Utc>,
}
pub fn validate(input: CustomerInput) -> Result<CustomerInput, AppError> {
    let mut errors = ValidationErrors::new();
    let code = input.code.trim().to_ascii_uppercase();
    let name = input.name.trim().to_owned();
    let email = input
        .email
        .filter(|v| !v.trim().is_empty())
        .map(|v| v.trim().to_owned());
    if code.is_empty()
        || code.len() > 32
        || !code.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
    {
        errors.insert(
            "code".into(),
            vec!["Use 1–32 letters, digits or hyphens.".into()],
        );
    }
    if name.is_empty() || name.encode_utf16().count() > 200 {
        errors.insert(
            "name".into(),
            vec!["A name of 1–200 characters is required.".into()],
        );
    }
    if email.as_ref().is_some_and(|v| {
        v.encode_utf16().count() > 254
            || v.matches('@').count() != 1
            || v.starts_with('@')
            || v.ends_with('@')
            || v.contains(['\r', '\n'])
    }) {
        errors.insert("email".into(), vec!["Enter a valid email address.".into()]);
    }
    if errors.is_empty() {
        Ok(CustomerInput {
            code,
            name,
            email,
            active: input.active,
        })
    } else {
        Err(AppError::Validation(errors))
    }
}
#[derive(Clone)]
pub struct CustomerService {
    db: Database,
}
fn dto(row: &DbRow) -> Result<CustomerDto, AppError> {
    Ok(CustomerDto {
        id: row.uuid("id").map_err(|e| e.application())?,
        code: row.text("code").map_err(|e| e.application())?,
        name: row.text("name").map_err(|e| e.application())?,
        email: row.optional_text("email").map_err(|e| e.application())?,
        active: row.boolean("active").map_err(|e| e.application())?,
        modified_at: row.timestamp("modified_at").map_err(|e| e.application())?,
    })
}
impl CustomerService {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
    pub async fn seed_customer(&self, input: CustomerInput) -> Result<CustomerDto, AppError> {
        let input = validate(input)?;
        let rows = self
            .db
            .query(
                Sql::new(
                    "SELECT * FROM crm.customers WHERE code=$1",
                    "SELECT * FROM crm.customers WHERE code=@P1",
                ),
                &[Value::Text(input.code.clone())],
            )
            .await
            .map_err(|e| e.application())?;
        if let Some(row) = rows.first() {
            if row.text("modified_by").map_err(|e| e.application())? != "development-seed" {
                return Err(AppError::Conflict{code:"demo_code_in_use".into(),message:"A demo customer code is already in use or was edited; existing records were preserved.".into()});
            }
            return dto(row);
        }
        Ok(self.save(None, input, None, "development-seed").await?.data)
    }
    pub async fn get(&self, id: Uuid) -> Result<Option<RecordResult<CustomerDto>>, AppError> {
        let rows = self
            .db
            .query(
                Sql::new(
                    "SELECT * FROM crm.customers WHERE id=$1",
                    "SELECT * FROM crm.customers WHERE id=@P1",
                ),
                &[Value::Uuid(id)],
            )
            .await
            .map_err(|e| e.application())?;
        rows.first()
            .map(|r| {
                Ok(RecordResult {
                    data: dto(r)?,
                    version: r.text("version").map_err(|e| e.application())?,
                    capabilities: None,
                })
            })
            .transpose()
    }
    pub async fn save(
        &self,
        id: Option<Uuid>,
        input: CustomerInput,
        expected: Option<&str>,
        actor: &str,
    ) -> Result<RecordResult<CustomerDto>, AppError> {
        let value = validate(input)?;
        let existing = if let Some(id) = id {
            let record = self.get(id).await?.ok_or(AppError::NotFound)?;
            if record.version != expected.unwrap_or_default() {
                return Err(AppError::VersionConflict);
            }
            Some(record)
        } else {
            None
        };
        let id = id.unwrap_or_else(Uuid::new_v4);
        let duplicates = self
            .db
            .query(
                Sql::new(
                    "SELECT id FROM crm.customers WHERE code=$1 AND id<>$2",
                    "SELECT id FROM crm.customers WHERE code=@P1 AND id<>@P2",
                ),
                &[Value::Text(value.code.clone()), Value::Uuid(id)],
            )
            .await
            .map_err(|e| e.application())?;
        if !duplicates.is_empty() {
            return Err(invalid("code", "This customer code already exists."));
        }
        let version = new_version();
        let modified_at = Utc::now().trunc_subsecs(6);
        let mut values = vec![
            Value::Uuid(id),
            Value::Text(value.code.clone()),
            Value::Text(value.name.clone()),
            Value::Text(search_key(&value.name)),
            Value::NullableText(value.email.clone()),
            Value::Bool(value.active),
            Value::Text(version.clone()),
            Value::Timestamp(modified_at),
            Value::Text(actor.into()),
        ];
        let changed=if let Some(existing)=existing {
   values.push(Value::Text(existing.version));
   self.db.execute(Sql::new("UPDATE crm.customers SET code=$2,name=$3,search_name=$4,email=$5,active=$6,version=$7,modified_at=$8,modified_by=$9 WHERE id=$1 AND version=$10","UPDATE crm.customers SET code=@P2,name=@P3,search_name=@P4,email=@P5,active=@P6,version=@P7,modified_at=@P8,modified_by=@P9 WHERE id=@P1 AND version=@P10"),&values).await
  }else{self.db.execute(Sql::new("INSERT INTO crm.customers(id,code,name,search_name,email,active,version,modified_at,modified_by) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9)","INSERT INTO crm.customers(id,code,name,search_name,email,active,version,modified_at,modified_by) VALUES(@P1,@P2,@P3,@P4,@P5,@P6,@P7,@P8,@P9)"),&values).await}.map_err(|e|e.application())?;
        if changed != 1 {
            return Err(AppError::VersionConflict);
        }
        Ok(RecordResult {
            data: CustomerDto {
                id,
                code: value.code,
                name: value.name,
                email: value.email,
                active: value.active,
                modified_at,
            },
            version,
            capabilities: None,
        })
    }
    pub async fn delete(&self, id: Uuid, expected: &str) -> Result<(), AppError> {
        let record = self.get(id).await?.ok_or(AppError::NotFound)?;
        if record.version != expected {
            return Err(AppError::VersionConflict);
        }
        let changed = self
            .db
            .execute(
                Sql::new(
                    "DELETE FROM crm.customers WHERE id=$1 AND version=$2",
                    "DELETE FROM crm.customers WHERE id=@P1 AND version=@P2",
                ),
                &[Value::Uuid(id), Value::Text(expected.into())],
            )
            .await
            .map_err(|e| e.application())?;
        if changed != 1 {
            return Err(AppError::VersionConflict);
        }
        Ok(())
    }
    pub async fn odata(&self, raw: Option<&str>) -> Result<serde_json::Value, AppError> {
        use bqatlas_odata::{DataType as T, Field};
        let fields = vec![
            Field::new("Id", "id", T::Guid),
            Field::new("Code", "code", T::String).search("code"),
            Field::new("Name", "name", T::String).search("search_name"),
            Field::new("Email", "email", T::String),
            Field::new("Active", "active", T::Boolean),
            Field::new("ModifiedAt", "modified_at", T::Timestamp),
        ];
        let query = bqatlas_odata::ReadQuery::parse(raw, &fields)?;
        bqatlas_odata::read(&self.db, "crm.customers", &fields, query, |row| {
            bqatlas_odata::pascal_projection(
                serde_json::to_value(dto(row)?).map_err(|_| AppError::Internal)?,
            )
        })
        .await
    }
    pub async fn query(
        &self,
        request: QueryRequest,
        lookup: bool,
    ) -> Result<PageResult<CustomerDto>, AppError> {
        validate_query(
            &request,
            if lookup {
                &[]
            } else {
                &["code", "name", "email", "active"]
            },
        )?;
        let builder = QuerySql::customers(&request, lookup)?;
        let count = self
            .db
            .query(
                Sql::new(
                    &format!(
                        "SELECT COUNT(*)::bigint AS total FROM crm.customers WHERE {}",
                        builder.postgres
                    ),
                    &format!(
                        "SELECT COUNT_BIG(*) AS total FROM crm.customers WHERE {}",
                        builder.sqlserver
                    ),
                ),
                &builder.values,
            )
            .await
            .map_err(|e| e.application())?;
        let total = count
            .first()
            .ok_or(AppError::Internal)?
            .integer("total")
            .map_err(|e| e.application())?;
        let order = if lookup {
            "code ASC,id ASC".into()
        } else {
            let mut order = vec![];
            for term in request.sort.iter().flatten() {
                let field = match term.field.as_str() {
                    "code" => "code",
                    "name" => "search_name",
                    "email" => "COALESCE(email,'')",
                    _ => "active",
                };
                order.push(format!("{field} {}", term.direction));
            }
            if order.is_empty() {
                order.push("code ASC".into());
            }
            order.push("id ASC".into());
            order.join(",")
        };
        let mut values = builder.values;
        let limit = values.len() + 1;
        let offset = limit + 1;
        values.push(Value::Int(request.page_size));
        values.push(Value::Int(request.page * request.page_size));
        let pg = format!(
            "SELECT * FROM crm.customers WHERE {} ORDER BY {order} LIMIT ${limit} OFFSET ${offset}",
            builder.postgres
        );
        let ms = format!(
            "SELECT * FROM crm.customers WHERE {} ORDER BY {order} OFFSET @P{offset} ROWS FETCH NEXT @P{limit} ROWS ONLY",
            builder.sqlserver
        );
        let rows = self
            .db
            .query(Sql::new(&pg, &ms), &values)
            .await
            .map_err(|e| e.application())?;
        Ok(PageResult {
            items: rows.iter().map(dto).collect::<Result<_, _>>()?,
            total,
            page: request.page,
            page_size: request.page_size,
        })
    }
}
#[async_trait]
impl CustomerDirectory for CustomerService {
    async fn find(&self, id: Uuid) -> Result<Option<CustomerSummary>, AppError> {
        Ok(self.get(id).await?.map(|record| CustomerSummary {
            id: record.data.id,
            code: record.data.code,
            name: record.data.name,
            active: record.data.active,
        }))
    }
}
struct QuerySql {
    postgres: String,
    sqlserver: String,
    values: Vec<Value>,
}
fn like_text(input: &str) -> String {
    input
        .chars()
        .flat_map(|c| {
            if matches!(c, '~' | '%' | '_' | '[') {
                vec!['~', c]
            } else {
                vec![c]
            }
        })
        .collect()
}
impl QuerySql {
    fn customers(query: &QueryRequest, lookup: bool) -> Result<Self, AppError> {
        let mut pg = vec![if lookup { "active=true" } else { "1=1" }.to_owned()];
        let mut ms = vec![if lookup { "active=1" } else { "1=1" }.to_owned()];
        let mut values = vec![];
        if !query.search.trim().is_empty() {
            values.push(Value::Text(format!(
                "%{}%",
                like_text(&search_key(query.search.trim()))
            )));
            let n = values.len();
            pg.push(format!(
                "(code LIKE ${n} ESCAPE '~' OR search_name LIKE ${n} ESCAPE '~')"
            ));
            ms.push(format!(
                "(code LIKE @P{n} ESCAPE '~' OR search_name LIKE @P{n} ESCAPE '~')"
            ));
        }
        for term in query.filters.iter().flatten() {
            if term.field == "active" {
                if term.operator != "eq"
                    || !matches!(term.value.trim().to_lowercase().as_str(), "true" | "false")
                {
                    return Err(invalid("filters", "Active accepts eq true or false."));
                }
                values.push(Value::Bool(term.value.trim().eq_ignore_ascii_case("true")));
                let n = values.len();
                pg.push(format!("active=${n}"));
                ms.push(format!("active=@P{n}"));
            } else {
                let field = match term.field.as_str() {
                    "code" => "code",
                    "name" => "search_name",
                    _ => "COALESCE(email,'')",
                };
                let text = if term.field == "email" {
                    term.value.clone()
                } else {
                    search_key(&term.value)
                };
                let value = match term.operator.as_str() {
                    "contains" => format!("%{}%", like_text(&text)),
                    "startsWith" => format!("{}%", like_text(&text)),
                    _ => text,
                };
                values.push(Value::Text(value));
                let n = values.len();
                if term.operator == "eq" {
                    pg.push(format!("{field}=${n}"));
                    ms.push(format!("{field}=@P{n}"));
                } else {
                    pg.push(format!("{field} LIKE ${n} ESCAPE '~'"));
                    ms.push(format!("{field} LIKE @P{n} ESCAPE '~'"));
                }
            }
        }
        Ok(Self {
            postgres: pg.join(" AND "),
            sqlserver: ms.join(" AND "),
            values,
        })
    }
}
pub fn router() -> Router<CustomerService> {
    Router::new()
        .route("/odata/Customers", get(odata_read))
        .route("/api/v1/crm/customers/query", post(query))
        .route("/api/v1/crm/customers/lookup", post(lookup))
        .route("/api/v1/crm/customers/lookup/{id}", get(resolve))
        .route("/api/v1/crm/customers", post(create))
        .route(
            "/api/v1/crm/customers/{id}",
            get(read).put(update).delete(delete),
        )
}
async fn query(
    State(service): State<CustomerService>,
    CurrentActor(actor): CurrentActor,
    ApiJson(request): ApiJson<QueryRequest>,
) -> ApiResult<Json<PageResult<CustomerDto>>> {
    actor.require(PERMISSIONS[0])?;
    Ok(Json(service.query(request, false).await?))
}
async fn lookup(
    State(service): State<CustomerService>,
    CurrentActor(actor): CurrentActor,
    ApiJson(request): ApiJson<QueryRequest>,
) -> ApiResult<Json<PageResult<CustomerSummary>>> {
    actor.require(CUSTOMER_LOOKUP)?;
    let result = service.query(request, true).await?;
    Ok(Json(PageResult {
        items: result
            .items
            .into_iter()
            .map(|r| CustomerSummary {
                id: r.id,
                code: r.code,
                name: r.name,
                active: r.active,
            })
            .collect(),
        total: result.total,
        page: result.page,
        page_size: result.page_size,
    }))
}
async fn resolve(
    State(service): State<CustomerService>,
    CurrentActor(actor): CurrentActor,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Option<CustomerSummary>>> {
    actor.require(CUSTOMER_LOOKUP)?;
    Ok(Json(service.find(id).await?.filter(|c| c.active)))
}
async fn read(
    State(service): State<CustomerService>,
    CurrentActor(actor): CurrentActor,
    Path(id): Path<Uuid>,
) -> ApiResult<Response> {
    actor.require(PERMISSIONS[0])?;
    record(service.get(id).await?.ok_or(AppError::NotFound)?, None)
}
async fn create(
    State(service): State<CustomerService>,
    CurrentActor(actor): CurrentActor,
    ApiJson(input): ApiJson<CustomerInput>,
) -> ApiResult<Response> {
    actor.require(PERMISSIONS[0])?;
    actor.require(PERMISSIONS[1])?;
    let result = service.save(None, input, None, &actor.id).await?;
    let path = format!("/api/v1/crm/customers/{}", result.data.id);
    record(result, Some(path))
}
async fn update(
    State(service): State<CustomerService>,
    CurrentActor(actor): CurrentActor,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    ApiJson(input): ApiJson<CustomerInput>,
) -> ApiResult<Response> {
    actor.require(PERMISSIONS[0])?;
    actor.require(PERMISSIONS[1])?;
    let version = expected_version(headers.get("If-Match").and_then(|v| v.to_str().ok()))?;
    record(
        service
            .save(Some(id), input, Some(version), &actor.id)
            .await?,
        None,
    )
}
async fn delete(
    State(service): State<CustomerService>,
    CurrentActor(actor): CurrentActor,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    actor.require(PERMISSIONS[0])?;
    actor.require(PERMISSIONS[2])?;
    let version = expected_version(headers.get("If-Match").and_then(|v| v.to_str().ok()))?;
    service.delete(id, version).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn odata_read(
    State(service): State<CustomerService>,
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
    fn customer_rules_normalize_and_reject_invalid_fields() {
        let value = validate(CustomerInput {
            code: " ac-01 ".into(),
            name: " Name ".into(),
            email: Some(" ".into()),
            active: true,
        })
        .expect("customer");
        assert_eq!(value.code, "AC-01");
        assert!(value.email.is_none());
        assert!(
            validate(CustomerInput {
                code: "bad code".into(),
                name: "".into(),
                email: Some("bad".into()),
                active: true
            })
            .is_err()
        );
    }
    #[test]
    fn search_wildcards_are_literals_and_user_sql_is_bound() {
        let sql = QuerySql::customers(
            &QueryRequest {
                search: "%_[' OR 1=1 --".into(),
                ..Default::default()
            },
            false,
        )
        .expect("query");
        assert!(!sql.postgres.contains("OR 1=1"));
        assert_eq!(sql.values.len(), 1);
        assert_eq!(like_text("%_[~"), "~%~_~[~~");
    }
}
