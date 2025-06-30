use crate::enums::FilterItemType;
use derive_builder::Builder;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Builder, Default, Eq, Hash)]
#[builder(setter(strip_option))]
pub struct FilterItem {
    pub filter_type: FilterItemType,
    pub name: String,
    pub value: String,
}
impl FilterItem {
    pub fn filter_type(&self) -> FilterItemType {
        self.filter_type
    }
    pub fn name(&self) -> String {
        self.name.clone()
    }
    pub fn value(&self) -> String {
        self.value.clone()
    }

    pub fn id(&self) -> String {
        match self.filter_type {
            FilterItemType::DueDate | FilterItemType::SECTION => format!("{:?}", self.filter_type),
            _ => format!("{:?}-{}", self.filter_type, self.value),
        }
    }
}
