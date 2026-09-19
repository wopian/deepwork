//! Sparse column history with a derived maximum, independent of underground width.
use serde::{Deserialize, Serialize};
use std::{
    cell::Cell,
    collections::BTreeMap,
    ops::{Deref, DerefMut},
};

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ColumnDepths {
    columns: BTreeMap<i64, i64>,
    #[serde(skip)]
    maximum: Cell<Option<i64>>,
}

impl ColumnDepths {
    pub fn deepest_row(&self) -> i64 {
        if let Some(value) = self.maximum.get() {
            return value;
        }
        let value = self.columns.values().max().copied().unwrap_or(0);
        self.maximum.set(Some(value));
        value
    }

    /// Excavation only increases column depth, so update the maximum without a scan.
    pub fn excavate_to(&mut self, x: i64, row: i64) {
        let maximum = self.deepest_row();
        self.columns
            .entry(x)
            .and_modify(|depth| *depth = (*depth).max(row))
            .or_insert(row);
        self.maximum.set(Some(maximum.max(row)));
    }
}

impl Deref for ColumnDepths {
    type Target = BTreeMap<i64, i64>;
    fn deref(&self) -> &Self::Target {
        &self.columns
    }
}
impl DerefMut for ColumnDepths {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Imports, fixtures and any arbitrary map mutation may lower the maximum.
        self.maximum.set(None);
        &mut self.columns
    }
}
impl FromIterator<(i64, i64)> for ColumnDepths {
    fn from_iter<T: IntoIterator<Item = (i64, i64)>>(iter: T) -> Self {
        Self {
            columns: iter.into_iter().collect(),
            maximum: Cell::new(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arbitrary_mutations_invalidate_derived_maximum() {
        let mut columns: ColumnDepths = [(-64, 400), (512, 800)].into_iter().collect();
        assert_eq!(columns.deepest_row(), 800);
        columns.insert(512, 200);
        assert_eq!(columns.deepest_row(), 400);
        columns.excavate_to(-1000, 1200);
        columns.excavate_to(-1000, 100);
        assert_eq!(columns.deepest_row(), 1200);
        assert_eq!(columns[&-1000], 1200);
        columns.remove(&-1000);
        assert_eq!(columns.deepest_row(), 400);
        columns.clear();
        assert_eq!(columns.deepest_row(), 0);
    }
    #[test]
    fn serialization_stays_a_plain_map_and_rebuilds_cache() {
        let mut columns = ColumnDepths::default();
        columns.excavate_to(-8, 96);
        let json = serde_json::to_string(&columns).unwrap();
        assert_eq!(json, r#"{"-8":96}"#);
        let restored: ColumnDepths = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.maximum.get(), None);
        assert_eq!(restored.deepest_row(), 96);
        let mut cloned = restored.clone();
        cloned.insert(-8, 4);
        assert_eq!(cloned.deepest_row(), 4);
        assert_eq!(restored.deepest_row(), 96);
    }
}
