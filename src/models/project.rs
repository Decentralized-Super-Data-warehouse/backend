use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Project {
    pub id: i32,
    pub name: String,
    pub token: String,
    pub category: String,
    pub contract_address: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub attributes: Vec<ProjectAttribute>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProjectAttribute {
    pub key: String,
    pub value: serde_json::Value,
}

impl Default for Project {
    fn default() -> Self {
        Project {
            id: 0,
            name: String::new(),
            token: String::new(),
            category: String::new(),
            contract_address: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            attributes: Vec::new(),
        }
    }
}

impl Project {
    pub fn get_attribute_value(&self, key: &str) -> Option<&serde_json::Value> {
        self.attributes
            .iter()
            .find(|attr| attr.key == key)
            .map(|attr| &attr.value)
    }

    pub fn set_attribute<T: Serialize>(
        &mut self,
        key: String,
        value: T,
    ) -> Result<(), serde_json::Error> {
        let json_value = serde_json::to_value(value)?;
        if let Some(attr) = self.attributes.iter_mut().find(|attr| attr.key == key) {
            attr.value = json_value;
        } else {
            self.attributes.push(ProjectAttribute {
                key,
                value: json_value,
            });
        }
        Ok(())
    }

    pub fn get_int(&self, key: &str) -> Option<i32> {
        self.get_attribute_value(key)
            .and_then(|v| v.as_i64()
            .map(|i| i as i32))
    }

    pub fn get_int64(&self, key: &str) -> Option<i64> {
        self.get_attribute_value(key).and_then(|v| v.as_i64())
    }

    pub fn get_float(&self, key: &str) -> Option<f64> {
        self.get_attribute_value(key).and_then(|v| v.as_f64())
    }

    pub fn get_string(&self, key: &str) -> Option<String> {
        self.get_attribute_value(key)
            .and_then(|v| v.as_str().map(String::from))
    }
}
