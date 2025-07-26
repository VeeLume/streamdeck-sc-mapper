use serde::{ Deserialize, Serialize };

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum DataSourceResult {
    Item(Item),
    ItemGroup(ItemGroup),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Item {
    pub value: String,
    pub label: Option<String>,
    pub disabled: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ItemGroup {
    pub label: String,
    pub children: Vec<Item>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataSourcePayload {
    pub event: Option<String>,
    pub items: Vec<DataSourceResult>,
}
