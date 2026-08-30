//! Feature store with versioning for persisting feature matrices.
//!
//! A [`ParquetFeatureStore`] backed by the `parquet` crate would provide
//! on-disk Arrow/Parquet persistence; this module ships [`InMemoryFeatureStore`]
//! as the default concrete implementation so the crate stays free of optional
//! Parquet dependencies.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Persistent storage for versioned feature matrices.
pub trait FeatureStore: Send + Sync {
    /// Save a feature matrix under `name` / `version`.
    fn save(
        &self,
        name: &str,
        version: &str,
        data: &[Vec<f64>],
        feature_names: &[String],
    ) -> Result<(), String>;

    /// Load a previously saved feature matrix.
    fn load(&self, name: &str, version: &str) -> Result<(Vec<Vec<f64>>, Vec<String>), String>;

    /// Append rows to an existing feature matrix.
    fn append(&self, name: &str, version: &str, data: &[Vec<f64>]) -> Result<(), String>;

    /// Remove a stored feature matrix version.
    fn invalidate(&self, name: &str, version: &str) -> Result<(), String>;

    /// List all versions stored for `name`.
    fn list_versions(&self, name: &str) -> Result<Vec<String>, String>;
}

#[derive(Clone, Debug, PartialEq)]
struct StoredFeature {
    data: Vec<Vec<f64>>,
    feature_names: Vec<String>,
}

fn validate_rows(data: &[Vec<f64>], feature_count: usize, context: &str) -> Result<(), String> {
    for (idx, row) in data.iter().enumerate() {
        if row.len() != feature_count {
            return Err(format!(
                "{context}: row {idx} has {} values, expected {feature_count}",
                row.len()
            ));
        }
    }
    Ok(())
}

/// Thread-safe in-memory [`FeatureStore`].
#[derive(Clone, Debug, Default)]
pub struct InMemoryFeatureStore {
    inner: Arc<Mutex<HashMap<String, HashMap<String, StoredFeature>>>>,
}

impl InMemoryFeatureStore {
    /// Create an empty in-memory feature store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl FeatureStore for InMemoryFeatureStore {
    fn save(
        &self,
        name: &str,
        version: &str,
        data: &[Vec<f64>],
        feature_names: &[String],
    ) -> Result<(), String> {
        validate_rows(data, feature_names.len(), "save")?;

        let mut store = self
            .inner
            .lock()
            .map_err(|_| "feature store lock poisoned".to_string())?;

        store
            .entry(name.to_string())
            .or_default()
            .insert(
                version.to_string(),
                StoredFeature {
                    data: data.to_vec(),
                    feature_names: feature_names.to_vec(),
                },
            );

        Ok(())
    }

    fn load(&self, name: &str, version: &str) -> Result<(Vec<Vec<f64>>, Vec<String>), String> {
        let store = self
            .inner
            .lock()
            .map_err(|_| "feature store lock poisoned".to_string())?;

        store
            .get(name)
            .and_then(|versions| versions.get(version))
            .map(|stored| (stored.data.clone(), stored.feature_names.clone()))
            .ok_or_else(|| format!("feature set '{name}' version '{version}' not found"))
    }

    fn append(&self, name: &str, version: &str, data: &[Vec<f64>]) -> Result<(), String> {
        let mut store = self
            .inner
            .lock()
            .map_err(|_| "feature store lock poisoned".to_string())?;

        let versions = store
            .get_mut(name)
            .ok_or_else(|| format!("feature set '{name}' version '{version}' not found"))?;

        let stored = versions
            .get_mut(version)
            .ok_or_else(|| format!("feature set '{name}' version '{version}' not found"))?;

        validate_rows(data, stored.feature_names.len(), "append")?;
        stored.data.extend(data.iter().cloned());

        Ok(())
    }

    fn invalidate(&self, name: &str, version: &str) -> Result<(), String> {
        let mut store = self
            .inner
            .lock()
            .map_err(|_| "feature store lock poisoned".to_string())?;

        let versions = store
            .get_mut(name)
            .ok_or_else(|| format!("feature set '{name}' version '{version}' not found"))?;

        if versions.remove(version).is_none() {
            return Err(format!("feature set '{name}' version '{version}' not found"));
        }

        if versions.is_empty() {
            store.remove(name);
        }

        Ok(())
    }

    fn list_versions(&self, name: &str) -> Result<Vec<String>, String> {
        let store = self
            .inner
            .lock()
            .map_err(|_| "feature store lock poisoned".to_string())?;

        let mut versions: Vec<String> = store
            .get(name)
            .map(|entries| entries.keys().cloned().collect())
            .unwrap_or_default();

        versions.sort_unstable();
        Ok(versions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn test_send_sync() {
        assert_send_sync::<InMemoryFeatureStore>();
    }

    #[test]
    fn test_save_and_load() {
        let store = InMemoryFeatureStore::new();
        let data = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let names = vec!["a".to_string(), "b".to_string()];

        store.save("features", "v1", &data, &names).unwrap();

        let (loaded_data, loaded_names) = store.load("features", "v1").unwrap();
        assert_eq!(loaded_data, data);
        assert_eq!(loaded_names, names);
    }

    #[test]
    fn test_append() {
        let store = InMemoryFeatureStore::new();
        let initial = vec![vec![1.0, 2.0]];
        let appended = vec![vec![3.0, 4.0], vec![5.0, 6.0]];
        let names = vec!["a".to_string(), "b".to_string()];

        store.save("features", "v1", &initial, &names).unwrap();
        store.append("features", "v1", &appended).unwrap();

        let (loaded_data, loaded_names) = store.load("features", "v1").unwrap();
        assert_eq!(
            loaded_data,
            vec![vec![1.0, 2.0], vec![3.0, 4.0], vec![5.0, 6.0]]
        );
        assert_eq!(loaded_names, names);
    }

    #[test]
    fn test_invalidate() {
        let store = InMemoryFeatureStore::new();
        let data = vec![vec![1.0]];
        let names = vec!["a".to_string()];

        store.save("features", "v1", &data, &names).unwrap();
        store.invalidate("features", "v1").unwrap();

        assert!(store.load("features", "v1").is_err());
        assert_eq!(store.list_versions("features").unwrap(), Vec::<String>::new());
    }

    #[test]
    fn test_list_versions() {
        let store = InMemoryFeatureStore::new();
        let data = vec![vec![1.0]];
        let names = vec!["a".to_string()];

        store.save("features", "v2", &data, &names).unwrap();
        store.save("features", "v1", &data, &names).unwrap();
        store.save("features", "v3", &data, &names).unwrap();

        assert_eq!(
            store.list_versions("features").unwrap(),
            vec!["v1".to_string(), "v2".to_string(), "v3".to_string()]
        );
    }
}
