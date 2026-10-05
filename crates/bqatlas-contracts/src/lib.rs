//! Stable bqAtlas HTTP contract v1. No HTTP framework or database dependency.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const CONTRACT_VERSION: &str = "1.0";
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FieldDescriptor {
    pub name: String,
    pub label: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub required: bool,
    pub max_length: Option<usize>,
    pub read_only: bool,
    pub options: Option<Vec<String>>,
    pub scale: Option<u32>,
    pub maximum: Option<String>,
}
impl FieldDescriptor {
    pub fn new(name: &str, label: &str, kind: &str, required: bool, max: Option<usize>) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            kind: kind.into(),
            required,
            max_length: max,
            read_only: false,
            options: None,
            scale: None,
            maximum: None,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceDescriptor {
    pub id: String,
    pub title: String,
    pub endpoint: String,
    pub read_permission: String,
    pub fields: Vec<FieldDescriptor>,
    pub key_field: String,
    pub o_data_endpoint: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationManifest {
    pub contract_version: String,
    pub modules: Vec<String>,
    pub resources: Vec<ResourceDescriptor>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub authenticated: bool,
    pub id: Option<String>,
    pub name: Option<String>,
    pub permissions: Vec<String>,
    pub auth_mode: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SortTerm {
    pub field: String,
    pub direction: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FilterTerm {
    pub field: String,
    pub operator: String,
    pub value: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct QueryRequest {
    pub page: i64,
    pub page_size: i64,
    pub search: String,
    pub sort: Option<Vec<SortTerm>>,
    pub filters: Option<Vec<FilterTerm>>,
}
impl Default for QueryRequest {
    fn default() -> Self {
        Self {
            page: 0,
            page_size: 25,
            search: String::new(),
            sort: None,
            filters: None,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageResult<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordCapabilities {
    pub edit: bool,
    pub delete: bool,
    pub commands: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordResult<T> {
    pub data: T,
    pub version: String,
    pub capabilities: Option<RecordCapabilities>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemDetails {
    #[serde(rename = "type")]
    pub kind: String,
    pub title: String,
    pub status: u16,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<BTreeMap<String, Vec<String>>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn contracts_use_frontend_names_and_preserve_nulls() {
        let input: QueryRequest =
            serde_json::from_str(r#"{"pageSize":10,"sort":null}"#).expect("query");
        assert_eq!(input.page_size, 10);
        assert_eq!(input.page, 0);
        let session = SessionInfo {
            authenticated: false,
            id: None,
            name: None,
            permissions: vec![],
            auth_mode: "local".into(),
        };
        let value = serde_json::to_value(session).expect("session");
        assert!(value["id"].is_null());
        assert_eq!(value["authMode"], "local");
        let field = serde_json::to_value(FieldDescriptor::new(
            "name",
            "Name",
            "string",
            true,
            Some(200),
        ))
        .expect("field");
        assert_eq!(field["maxLength"], 200);
        assert_eq!(field["type"], "string");
        assert!(field["options"].is_null());
    }
}
