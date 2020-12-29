use std::collections::HashMap;

#[derive(Clone)]
pub struct ConfigMap {
    data: HashMap<String, String>,
}

impl Default for ConfigMap {
    fn default() -> Self {
        ConfigMap {
            data: HashMap::new(),
        }
    }
}

impl ConfigMap {
    pub fn new() -> ConfigMap {
        Default::default()
    }

    pub fn flag<T: AsRef<str>>(&self, key: T) -> bool {
        if !self.data.contains_key(key.as_ref()) {
            return false;
        }
        self.is_true(&self.data[key.as_ref()])
    }

    fn is_true(&self, value: &str) -> bool {
        let value_lower = value.to_lowercase();
        value_lower == "1" || value_lower == "yes" || value_lower == "true"
    }

    pub fn set<A: AsRef<str>, B: AsRef<str>>(&mut self, key: A, value: B) {
        let _ = self
            .data
            .insert(key.as_ref().to_string(), value.as_ref().to_string());
    }

    pub fn import(&mut self, settings: &HashMap<String, String>) {
        for (key, value) in settings.iter() {
            self.set(key, value)
        }
    }

    pub fn get_string<T: AsRef<str>>(&self, key: T) -> Option<String> {
        if !self.data.contains_key(key.as_ref()) {
            return None;
        }
        Some(self.data[key.as_ref()].to_string())
    }

    pub fn get_u32<T: AsRef<str>>(&self, key: T) -> Option<u32> {
        match str::parse::<u32>(&self.data[key.as_ref()]) {
            Ok(v) => Some(v),
            Err(_) => None,
        }
    }
}
