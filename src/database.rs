use std::{collections::HashMap, time};

pub type Key = String;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Value {
    pub data: String,
    ttl: Option<time::Duration>,
    created: time::Instant,
}

impl Value {
    pub fn new(data: String, ttl: Option<time::Duration>) -> Self {
        Self {
            data,
            ttl,
            created: time::Instant::now(),
        }
    }

    #[allow(dead_code)]
    pub fn without_ttl(data: String) -> Self {
        Self {
            data,
            ttl: None,
            created: time::Instant::now(),
        }
    }

    #[allow(dead_code)]
    pub fn with_ttl(data: String, ttl: time::Duration) -> Self {
        Self {
            data,
            ttl: Some(ttl),
            created: time::Instant::now(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("No value is associated with such key")]
    KeyNotFound,
    #[error("This key-value pair has expired")]
    Expired,
}

#[derive(Debug, Clone)]
pub struct Database {
    storage: HashMap<Key, Value>,
}

impl Database {
    pub fn new() -> Self {
        Self {
            storage: HashMap::new(),
        }
    }

    pub fn get(&self, key: &str) -> Result<&Value, Error> {
        let now = time::Instant::now();
        let value = self.storage.get(key).ok_or(Error::KeyNotFound)?;
        match value.ttl {
            Some(ttl) if now.duration_since(value.created) > ttl => Err(Error::Expired),
            _ => Ok(value),
        }
    }

    pub fn set(&mut self, key: String, value: Value) {
        let _ = self.storage.insert(key, value);
    }
}

#[cfg(test)]
mod tests {
    use crate::database::{Database, Error, Value};
    use std::{thread, time::Duration};

    #[test]
    fn no_ttl() {
        let mut db = Database::new();
        db.set("foo".into(), Value::without_ttl("bar".into()));
        assert_eq!(db.get("foo").unwrap().data, "bar");
    }

    #[test]
    fn with_ttl() {
        let mut db = Database::new();
        db.set(
            "foo".into(),
            Value::with_ttl("bar".into(), Duration::from_millis(10)),
        );
        db.set(
            "bar".into(),
            Value::with_ttl("baz".into(), Duration::from_secs(1)),
        );
        thread::sleep(Duration::from_millis(20));
        assert_eq!(db.get("foo"), Err(Error::Expired));
        assert_eq!(db.get("bar").unwrap().data, "baz");
    }
}
