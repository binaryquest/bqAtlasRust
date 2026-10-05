//! Bounded read query adapter, not a complete OData protocol implementation.
//! Identifiers come from module-owned field maps; every literal is bound as a driver parameter.
use bqatlas_core::{AppError, search_key};
use bqatlas_db::{Database, DbRow, Sql, Value};
use serde_json::{Value as Json, json};
use std::collections::BTreeSet;
use std::str::FromStr;
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DataType {
    String,
    Boolean,
    Guid,
    Date,
    Decimal,
    Integer,
    Timestamp,
}
#[derive(Clone)]
pub struct Field {
    pub property: String,
    pub column: String,
    pub kind: DataType,
    pub search_column: Option<String>,
}
impl Field {
    pub fn new(property: &str, column: &str, kind: DataType) -> Self {
        Self {
            property: property.into(),
            column: column.into(),
            kind,
            search_column: None,
        }
    }
    pub fn search(mut self, column: &str) -> Self {
        self.search_column = Some(column.into());
        self
    }
}
#[derive(Clone, Debug, PartialEq)]
enum Token {
    Word(String),
    Text(String),
    Open,
    Close,
    Comma,
}
fn bad() -> AppError {
    AppError::BadRequest(
        "Unsupported or invalid OData query. See docs/ODATA.md for the bounded subset.".into(),
    )
}
fn tokens(input: &str) -> Result<Vec<Token>, AppError> {
    if input.len() > 4096 {
        return Err(bad());
    }
    let mut chars = input.chars().peekable();
    let mut out = vec![];
    while let Some(c) = chars.next() {
        let token = match c {
            c if c.is_whitespace() => continue,
            '(' => Token::Open,
            ')' => Token::Close,
            ',' => Token::Comma,
            '\'' => {
                let mut value = String::new();
                let mut closed = false;
                while let Some(c) = chars.next() {
                    if c == '\'' {
                        if chars.peek() == Some(&'\'') {
                            chars.next();
                            value.push('\'');
                        } else {
                            closed = true;
                            break;
                        }
                    } else {
                        value.push(c);
                    }
                }
                if !closed || value.encode_utf16().count() > 200 {
                    return Err(bad());
                }
                Token::Text(value)
            }
            c if c.is_ascii_alphanumeric()
                || c == '_'
                || (c == '-' && chars.peek().is_some_and(|c| c.is_ascii_digit())) =>
            {
                let mut value = c.to_string();
                while chars.peek().is_some_and(|c| {
                    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ':' | '+')
                }) {
                    value.push(chars.next().ok_or_else(bad)?);
                }
                Token::Word(value)
            }
            _ => return Err(bad()),
        };
        out.push(token);
        if out.len() > 160 {
            return Err(bad());
        }
    }
    Ok(out)
}
#[derive(Debug)]
enum Expr {
    Eq(usize, Option<Value>),
    Text(usize, String, bool),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
}
struct Parser<'a> {
    tokens: Vec<Token>,
    at: usize,
    fields: &'a [Field],
    terms: usize,
}
impl Parser<'_> {
    fn next(&mut self) -> Result<Token, AppError> {
        let t = self.tokens.get(self.at).cloned().ok_or_else(bad)?;
        self.at += 1;
        Ok(t)
    }
    fn take(&mut self, token: Token) -> bool {
        if self.tokens.get(self.at) == Some(&token) {
            self.at += 1;
            true
        } else {
            false
        }
    }
    fn field(&self, name: &str) -> Result<usize, AppError> {
        self.fields
            .iter()
            .position(|f| f.property == name)
            .ok_or_else(bad)
    }
    fn word(&mut self) -> Result<String, AppError> {
        match self.next()? {
            Token::Word(w) => Ok(w),
            _ => Err(bad()),
        }
    }
    fn expression(&mut self, depth: usize) -> Result<Expr, AppError> {
        let mut left = self.and(depth)?;
        while self.take(Token::Word("or".into())) {
            left = Expr::Or(Box::new(left), Box::new(self.and(depth)?));
        }
        Ok(left)
    }
    fn and(&mut self, depth: usize) -> Result<Expr, AppError> {
        let mut left = self.atom(depth)?;
        while self.take(Token::Word("and".into())) {
            left = Expr::And(Box::new(left), Box::new(self.atom(depth)?));
        }
        Ok(left)
    }
    fn atom(&mut self, depth: usize) -> Result<Expr, AppError> {
        if depth > 8 {
            return Err(bad());
        }
        if self.take(Token::Open) {
            let exp = self.expression(depth + 1)?;
            if !self.take(Token::Close) {
                return Err(bad());
            }
            return Ok(exp);
        }
        self.terms += 1;
        if self.terms > 30 {
            return Err(bad());
        }
        let name = self.word()?;
        if name == "contains" || name == "startswith" {
            if !self.take(Token::Open) {
                return Err(bad());
            }
            let field_name = self.word()?;
            let index = self.field(&field_name)?;
            if self.fields[index].kind != DataType::String || !self.take(Token::Comma) {
                return Err(bad());
            }
            let value = match self.next()? {
                Token::Text(value) => value,
                _ => return Err(bad()),
            };
            if !self.take(Token::Close) {
                return Err(bad());
            }
            return Ok(Expr::Text(index, value, name == "startswith"));
        }
        let index = self.field(&name)?;
        if self.word()? != "eq" {
            return Err(bad());
        }
        let value = self.next()?;
        if value == Token::Word("null".into()) {
            return Ok(Expr::Eq(index, None));
        }
        let value = match (self.fields[index].kind, value) {
            (DataType::String, Token::Text(value)) => Value::Text(search_key(&value)),
            (DataType::Boolean, Token::Word(value)) => Value::Bool(match value.as_str() {
                "true" => true,
                "false" => false,
                _ => return Err(bad()),
            }),
            (DataType::Guid, Token::Word(value)) => {
                Value::Uuid(uuid::Uuid::parse_str(&value).map_err(|_| bad())?)
            }
            (DataType::Date, Token::Word(value)) => Value::Date(
                chrono::NaiveDate::parse_from_str(&value, "%Y-%m-%d").map_err(|_| bad())?,
            ),
            (DataType::Decimal, Token::Word(value)) => {
                if value.len() > 32
                    || value
                        .bytes()
                        .enumerate()
                        .any(|(i, b)| !b.is_ascii_digit() && b != b'.' && !(i == 0 && b == b'-'))
                {
                    return Err(bad());
                }
                Value::Decimal(rust_decimal::Decimal::from_str(&value).map_err(|_| bad())?)
            }
            (DataType::Integer, Token::Word(value)) => {
                Value::Int(value.parse().map_err(|_| bad())?)
            }
            (DataType::Timestamp, Token::Word(value)) => Value::Timestamp(
                chrono::DateTime::parse_from_rfc3339(&value)
                    .map_err(|_| bad())?
                    .to_utc(),
            ),
            _ => return Err(bad()),
        };
        Ok(Expr::Eq(index, Some(value)))
    }
}
pub struct ReadQuery {
    top: i64,
    skip: i64,
    count: bool,
    order: Vec<(usize, bool)>,
    filter: Option<Expr>,
}
impl ReadQuery {
    pub fn parse(raw: Option<&str>, fields: &[Field]) -> Result<Self, AppError> {
        let pairs: Vec<(String, String)> =
            serde_urlencoded::from_str(raw.unwrap_or_default()).map_err(|_| bad())?;
        let mut names = BTreeSet::new();
        let mut query = Self {
            top: 25,
            skip: 0,
            count: false,
            order: vec![],
            filter: None,
        };
        for (key, value) in pairs {
            if !names.insert(key.clone()) {
                return Err(bad());
            }
            match key.as_str() {
                "$top" => query.top = value.parse().map_err(|_| bad())?,
                "$skip" => query.skip = value.parse().map_err(|_| bad())?,
                "$count" => {
                    query.count = match value.as_str() {
                        "true" => true,
                        "false" => false,
                        _ => return Err(bad()),
                    }
                }
                "$orderby" => {
                    let mut used = BTreeSet::new();
                    for term in value.split(',') {
                        let parts: Vec<_> = term.split_whitespace().collect();
                        if parts.is_empty() || parts.len() > 2 {
                            return Err(bad());
                        }
                        let index = fields
                            .iter()
                            .position(|f| f.property == parts[0])
                            .ok_or_else(bad)?;
                        if !used.insert(index) {
                            return Err(bad());
                        }
                        let desc = match parts.get(1).copied().unwrap_or("asc") {
                            "asc" => false,
                            "desc" => true,
                            _ => return Err(bad()),
                        };
                        query.order.push((index, desc));
                    }
                    if query.order.len() > 5 {
                        return Err(bad());
                    }
                }
                "$filter" => {
                    let mut parser = Parser {
                        tokens: tokens(&value)?,
                        at: 0,
                        fields,
                        terms: 0,
                    };
                    let exp = parser.expression(0)?;
                    if parser.at != parser.tokens.len() {
                        return Err(bad());
                    }
                    query.filter = Some(exp);
                }
                _ => return Err(bad()),
            }
        }
        if !(0..=100).contains(&query.top) || !(0..=100000).contains(&query.skip) {
            return Err(bad());
        }
        Ok(query)
    }
}
fn text_column(field: &Field) -> String {
    field
        .search_column
        .clone()
        .unwrap_or_else(|| format!("UPPER({})", field.column))
}
fn predicate(exp: &Expr, fields: &[Field], values: &mut Vec<Value>, postgres: bool) -> String {
    match exp {
        Expr::And(a, b) | Expr::Or(a, b) => format!(
            "({} {} {})",
            predicate(a, fields, values, postgres),
            if matches!(exp, Expr::And(..)) {
                "AND"
            } else {
                "OR"
            },
            predicate(b, fields, values, postgres)
        ),
        Expr::Eq(index, value) => {
            let f = &fields[*index];
            let column = if f.kind == DataType::String {
                text_column(f)
            } else {
                f.column.clone()
            };
            match value {
                None => format!("{} IS NULL", f.column),
                Some(value) => {
                    values.push(value.clone());
                    format!(
                        "{column}={}",
                        if postgres {
                            format!("${}", values.len())
                        } else {
                            format!("@P{}", values.len())
                        }
                    )
                }
            }
        }
        Expr::Text(index, value, starts) => {
            let value = search_key(value)
                .replace('~', "~~")
                .replace('%', "~%")
                .replace('_', "~_")
                .replace('[', "~[");
            values.push(Value::Text(if *starts {
                format!("{value}%")
            } else {
                format!("%{value}%")
            }));
            format!(
                "{} LIKE {} ESCAPE '~'",
                text_column(&fields[*index]),
                if postgres {
                    format!("${}", values.len())
                } else {
                    format!("@P{}", values.len())
                }
            )
        }
    }
}
/// Called inside the module that owns the table and its public projection.
pub async fn read(
    db: &Database,
    table: &str,
    fields: &[Field],
    query: ReadQuery,
    project: impl Fn(&DbRow) -> Result<Json, AppError>,
) -> Result<Json, AppError> {
    // Maps/table names are code-owned, never accepted from request input.
    if !table
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.'))
    {
        return Err(AppError::Internal);
    }
    let mut pg_values = vec![];
    let mut ms_values = vec![];
    let pg = query.filter.as_ref().map_or_else(
        || "1=1".into(),
        |f| predicate(f, fields, &mut pg_values, true),
    );
    let ms = query.filter.as_ref().map_or_else(
        || "1=1".into(),
        |f| predicate(f, fields, &mut ms_values, false),
    );
    let mut values = pg_values;
    let total = if query.count {
        let rows = db
            .query(
                Sql::new(
                    &format!("SELECT COUNT(*)::bigint AS total FROM {table} WHERE {pg}"),
                    &format!("SELECT COUNT_BIG(*) AS total FROM {table} WHERE {ms}"),
                ),
                &values,
            )
            .await
            .map_err(|e| e.application())?;
        Some(rows[0].integer("total").map_err(|e| e.application())?)
    } else {
        None
    };
    let mut order = query
        .order
        .iter()
        .map(|(i, desc)| {
            format!(
                "{} {}",
                fields[*i].column,
                if *desc { "DESC" } else { "ASC" }
            )
        })
        .collect::<Vec<_>>();
    if !query.order.iter().any(|(i, _)| fields[*i].column == "id") {
        order.push("id ASC".into());
    }
    let order = order.join(",");
    let limit = values.len() + 1;
    let offset = limit + 1;
    values.push(Value::Int(query.top));
    values.push(Value::Int(query.skip));
    let rows = if query.top == 0 {
        vec![]
    } else {
        db.query(Sql::new(&format!("SELECT * FROM {table} WHERE {pg} ORDER BY {order} LIMIT ${limit} OFFSET ${offset}"),&format!("SELECT * FROM {table} WHERE {ms} ORDER BY {order} OFFSET @P{offset} ROWS FETCH NEXT @P{limit} ROWS ONLY")),&values).await.map_err(|e|e.application())?
    };
    let items = rows.iter().map(project).collect::<Result<Vec<_>, _>>()?;
    let mut response = json!({"value":items});
    if let Some(total) = total {
        response["@odata.count"] = json!(total.to_string());
    }
    Ok(response)
}
pub fn pascal_projection(dto: Json) -> Result<Json, AppError> {
    let object = dto.as_object().ok_or(AppError::Internal)?;
    let mut out = serde_json::Map::new();
    for (key, value) in object {
        let mut chars = key.chars();
        let first = chars.next().ok_or(AppError::Internal)?.to_ascii_uppercase();
        out.insert(format!("{first}{}", chars.as_str()), value.clone());
    }
    Ok(Json::Object(out))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fields() -> Vec<Field> {
        vec![
            Field::new("Name", "name", DataType::String),
            Field::new("Active", "active", DataType::Boolean),
            Field::new("Id", "id", DataType::Guid),
        ]
    }
    #[test]
    fn grouping_literals_and_parameter_binding() {
        let fields = fields();
        let raw = serde_urlencoded::to_string([
            (
                "$filter",
                "(contains(Name,'O''Brien%') or startswith(Name,'X')) and Active eq true",
            ),
            ("$orderby", "Name desc,Id"),
            ("$count", "true"),
        ])
        .expect("encode");
        let query = ReadQuery::parse(Some(&raw), &fields).expect("query");
        let mut values = vec![];
        let sql = predicate(
            query.filter.as_ref().expect("filter"),
            &fields,
            &mut values,
            true,
        );
        assert!(!sql.contains("Brien"));
        assert!(sql.contains("OR"));
        assert!(sql.contains("$3"));
        assert!(matches!(&values[0],Value::Text(v) if v=="%O'BRIEN~%%"));
    }
    #[test]
    fn signed_numbers_and_offset_timestamps_remain_typed_parameters() {
        let fields = vec![
            Field::new("Price", "price", DataType::Decimal),
            Field::new("Count", "count", DataType::Integer),
            Field::new("CreatedAt", "created_at", DataType::Timestamp),
        ];
        let raw = serde_urlencoded::to_string([(
            "$filter",
            "Price eq -12.50 and Count eq -2 and CreatedAt eq 2026-09-30T12:00:00+06:00",
        )])
        .expect("encoded filter");
        let query = ReadQuery::parse(Some(&raw), &fields).expect("typed query");
        let mut values = vec![];
        let sql = predicate(
            query.filter.as_ref().expect("filter"),
            &fields,
            &mut values,
            true,
        );
        assert!(!sql.contains("12.50"));
        assert!(
            matches!(&values[0], Value::Decimal(v) if *v == rust_decimal::Decimal::new(-1250, 2))
        );
        assert!(matches!(&values[1], Value::Int(-2)));
        assert!(
            matches!(&values[2], Value::Timestamp(v) if v.to_rfc3339() == "2026-09-30T06:00:00+00:00")
        );
    }
    #[test]
    fn unsafe_and_excessive_queries_are_rejected() {
        let fields = fields();
        for raw in [
            "$expand=Customer",
            "$select=password_hash",
            "$top=101",
            "$skip=100001",
            "$top=1&$top=2",
            "$orderby=password_hash",
            "$filter=Active%20eq%20'yes'",
            "$filter=Name%20eq%20'bad'%20or%201%20eq%201",
            "$filter=contains(Name,'unterminated)",
        ] {
            assert!(
                ReadQuery::parse(Some(raw), &fields).is_err(),
                "accepted {raw}"
            );
        }
        let deep = format!(
            "$filter={}Name%20eq%20'x'{}",
            "(".repeat(10),
            ")".repeat(10)
        );
        assert!(ReadQuery::parse(Some(&deep), &fields).is_err());
    }
}
