use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum DataSourceResult {
    Item(Item),
    ItemGroup(ItemGroup),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Item {
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ItemGroup {
    pub label: String,
    pub children: Vec<Item>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataSourcePayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    pub items: Vec<DataSourceResult>,
}

// ---- helpers (optional) -----------------------------------------------------

impl Item {
    pub fn _new<V: Into<String>>(value: V) -> Self {
        Self { value: value.into(), label: None, disabled: None }
    }
    pub fn with_label<V: Into<String>, L: Into<String>>(value: V, label: L) -> Self {
        Self { value: value.into(), label: Some(label.into()), disabled: None }
    }
    pub fn _disabled(mut self, v: bool) -> Self {
        if v { self.disabled = Some(true); }
        self
    }
}

impl ItemGroup {
    pub fn new<L: Into<String>>(label: L, children: Vec<Item>) -> Self {
        Self { label: label.into(), children }
    }
}
