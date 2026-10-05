//! Framework policy and application contracts, independent of drivers and HTTP.
use bqatlas_contracts::{QueryRequest, ResourceDescriptor};
use rust_decimal::Decimal;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use unicode_normalization::UnicodeNormalization;

pub type ValidationErrors = BTreeMap<String, Vec<String>>;
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Validation failed")]
    Validation(ValidationErrors),
    #[error("{0}")]
    BadRequest(String),
    #[error("Authentication is required")]
    Unauthorized,
    #[error("This operation is not permitted")]
    Forbidden,
    #[error("Record was not found")]
    NotFound,
    #[error("If-Match is required.")]
    PreconditionRequired,
    #[error("This record changed after you opened it. Reload before saving.")]
    VersionConflict,
    #[error("{message}")]
    Conflict { code: String, message: String },
    #[error("Invalid request verification token")]
    Csrf,
    #[error("Too many requests. Try again later.")]
    RateLimited,
    #[error("Service unavailable")]
    Unavailable,
    #[error("An unexpected error occurred")]
    Internal,
}
pub fn invalid(field: &str, message: &str) -> AppError {
    AppError::Validation(BTreeMap::from([(field.into(), vec![message.into()])]))
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Actor {
    pub id: String,
    pub name: String,
    pub permissions: BTreeSet<String>,
}
impl Actor {
    pub fn require(&self, permission: &str) -> Result<(), AppError> {
        if self.permissions.contains(permission) {
            Ok(())
        } else {
            Err(AppError::Forbidden)
        }
    }
    pub fn external_id(issuer: &str, subject: &str) -> String {
        format!(
            "oidc:{:x}",
            Sha256::digest(format!("{issuer}\n{subject}").as_bytes())
        )
    }
}
pub fn new_version() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}
pub fn expected_version(header: Option<&str>) -> Result<&str, AppError> {
    let value = header.ok_or(AppError::PreconditionRequired)?;
    if value.len() == 34 && value.starts_with('"') && value.ends_with('"') {
        let token = &value[1..33];
        if token.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Ok(token);
        }
    }
    if value.len() == 32 && value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Ok(value);
    }
    Err(AppError::VersionConflict)
}
pub fn search_key(input: &str) -> String {
    // Match invariant simple uppercase: do not expand ß into SS.
    input
        .nfc()
        .map(|c| {
            let mut upper = c.to_uppercase();
            let first = upper.next().unwrap_or(c);
            if upper.next().is_some() { c } else { first }
        })
        .collect()
}
pub fn validate_query(query: &QueryRequest, fields: &[&str]) -> Result<(), AppError> {
    let mut errors = ValidationErrors::new();
    if !(0..=100_000).contains(&query.page) {
        errors.insert(
            "page".into(),
            vec!["Page must be between 0 and 100000.".into()],
        );
    }
    if !(1..=100).contains(&query.page_size) {
        errors.insert(
            "pageSize".into(),
            vec!["Page size must be between 1 and 100.".into()],
        );
    }
    if query.search.encode_utf16().count() > 200 {
        errors.insert(
            "search".into(),
            vec!["Search is limited to 200 characters.".into()],
        );
    }
    if query.sort.as_ref().is_some_and(|x| x.len() > 5)
        || query.filters.as_ref().is_some_and(|x| x.len() > 10)
    {
        errors.insert(
            "query".into(),
            vec!["Too many sort or filter terms.".into()],
        );
    }
    for sort in query.sort.iter().flatten() {
        if !fields.contains(&sort.field.as_str())
            || !matches!(sort.direction.as_str(), "asc" | "desc")
        {
            errors.insert(
                "sort".into(),
                vec!["Unsupported sort field or direction.".into()],
            );
        }
    }
    for filter in query.filters.iter().flatten() {
        if !fields.contains(&filter.field.as_str())
            || !matches!(filter.operator.as_str(), "eq" | "contains" | "startsWith")
            || filter.value.encode_utf16().count() > 200
        {
            errors.insert(
                "filters".into(),
                vec!["Unsupported filter field, operator or value.".into()],
            );
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(AppError::Validation(errors))
    }
}
pub fn decimal_text(text: &str, scale: u32, maximum: Decimal) -> Option<Decimal> {
    if text.is_empty() || text.len() > 30 || scale > 28 {
        return None;
    }
    let (whole, fraction) = text
        .split_once('.')
        .map_or((text, None), |(w, f)| (w, Some(f)));
    if whole.is_empty()
        || (whole.len() > 1 && whole.starts_with('0'))
        || !whole.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    if fraction.is_some_and(|f| {
        f.is_empty() || f.len() > scale as usize || !f.bytes().all(|b| b.is_ascii_digit())
    }) {
        return None;
    }
    let parsed = text.parse::<Decimal>().ok()?;
    (parsed <= maximum).then_some(parsed)
}
#[derive(Debug, Clone)]
pub struct ModuleDefinition {
    pub id: String,
    pub dependencies: Vec<String>,
}
#[derive(Debug, Clone, Default)]
pub struct Registry {
    modules: BTreeMap<String, ModuleDefinition>,
    resources: BTreeMap<String, ResourceDescriptor>,
}
impl Registry {
    pub fn add_module(&mut self, module: ModuleDefinition) -> Result<(), String> {
        if self.modules.contains_key(&module.id) {
            return Err(format!("Duplicate module {}", module.id));
        }
        self.modules.insert(module.id.clone(), module);
        Ok(())
    }
    pub fn add_resource(&mut self, resource: ResourceDescriptor) -> Result<(), String> {
        if self.resources.contains_key(&resource.id) {
            return Err(format!("Duplicate resource {}", resource.id));
        }
        self.resources.insert(resource.id.clone(), resource);
        Ok(())
    }
    pub fn module_order(&self) -> Result<Vec<String>, String> {
        let mut pending = self.modules.clone();
        let mut ready = Vec::new();
        while !pending.is_empty() {
            let next = pending
                .values()
                .find(|m| m.dependencies.iter().all(|d| ready.contains(d)))
                .map(|m| m.id.clone());
            let Some(id) = next else {
                return Err("Missing dependency or module cycle".into());
            };
            pending.remove(&id);
            ready.push(id);
        }
        Ok(ready)
    }
    pub fn resources_for(&self, actor: &Actor) -> Vec<ResourceDescriptor> {
        self.resources
            .values()
            .filter(|r| actor.permissions.contains(&r.read_permission))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn modules_reject_missing_dependencies_cycles_and_duplicates() {
        let mut r = Registry::default();
        r.add_module(ModuleDefinition {
            id: "sales".into(),
            dependencies: vec!["crm".into()],
        })
        .expect("sales");
        assert!(r.module_order().is_err());
        r.add_module(ModuleDefinition {
            id: "crm".into(),
            dependencies: vec![],
        })
        .expect("crm");
        assert_eq!(r.module_order().expect("order"), vec!["crm", "sales"]);
        assert!(
            r.add_module(ModuleDefinition {
                id: "crm".into(),
                dependencies: vec![]
            })
            .is_err()
        );
        let mut cycle = Registry::default();
        cycle
            .add_module(ModuleDefinition {
                id: "a".into(),
                dependencies: vec!["b".into()],
            })
            .expect("a");
        cycle
            .add_module(ModuleDefinition {
                id: "b".into(),
                dependencies: vec!["a".into()],
            })
            .expect("b");
        assert!(cycle.module_order().is_err());
    }
    #[test]
    fn decimal_grammar_preserves_portable_precision() {
        let maximum = Decimal::from(1_000_000);
        for bad in [
            "", "01", "-1", "+1", "1e2", "1,000", "1.", ".5", "1.0001", "1000001",
        ] {
            assert!(decimal_text(bad, 3, maximum).is_none(), "{bad}");
        }
        assert_eq!(
            decimal_text("999999.999", 3, maximum)
                .expect("decimal")
                .to_string(),
            "999999.999"
        );
    }
    #[test]
    fn queries_and_preconditions_are_bounded() {
        assert!(
            validate_query(
                &QueryRequest {
                    page_size: 101,
                    ..Default::default()
                },
                &[]
            )
            .is_err()
        );
        assert!(
            validate_query(
                &QueryRequest {
                    sort: Some(vec![bqatlas_contracts::SortTerm {
                        field: "code; DROP TABLE x".into(),
                        direction: "asc".into()
                    }]),
                    ..Default::default()
                },
                &["code"]
            )
            .is_err()
        );
        let version = new_version();
        assert_eq!(
            expected_version(Some(&format!("\"{version}\""))).expect("etag"),
            version
        );
        assert!(expected_version(None).is_err());
        assert!(expected_version(Some("*")).is_err());
        assert_eq!(search_key("Cafe\u{301}"), "CAFÉ");
    }
}
