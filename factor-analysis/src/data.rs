use crate::error::{ResearchError, ResearchResult};
use finkit::features::{Feature, FeatureMatrix};
use finkit::math::segmented::SegmentLayout;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Dictionary-encoded asset identifier used on research hot paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AssetId(pub u32);

/// Dictionary-encoded categorical group identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GroupId(pub u32);

/// Sorted `(timestamp, asset)` panel index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelIndex {
    timestamps: Vec<i64>,
    assets: Vec<AssetId>,
    date_segments: SegmentLayout,
}

impl PanelIndex {
    pub fn new(timestamps: Vec<i64>, assets: Vec<AssetId>) -> ResearchResult<Self> {
        if timestamps.len() != assets.len() {
            return Err(ResearchError::LengthMismatch {
                name: "assets".to_string(),
                expected: timestamps.len(),
                actual: assets.len(),
            });
        }
        if timestamps.windows(2).any(|w| w[0] > w[1]) {
            return Err(ResearchError::InvalidConfig(
                "PanelIndex timestamps must be sorted ascending".to_string(),
            ));
        }
        let date_segments = SegmentLayout::from_sorted_keys(&timestamps);
        Ok(Self {
            timestamps,
            assets,
            date_segments,
        })
    }

    pub fn len(&self) -> usize {
        self.timestamps.len()
    }
    pub fn is_empty(&self) -> bool {
        self.timestamps.is_empty()
    }
    pub fn timestamps(&self) -> &[i64] {
        &self.timestamps
    }
    pub fn assets(&self) -> &[AssetId] {
        &self.assets
    }
    pub fn date_segments(&self) -> &SegmentLayout {
        &self.date_segments
    }
    pub fn dates(&self) -> Vec<i64> {
        self.date_segments.map(|range| self.timestamps[range.start])
    }
}

/// Numeric feature/factor columns plus panel and categorical metadata.
#[derive(Debug, Clone)]
pub struct ResearchFrame {
    index: PanelIndex,
    numeric: FeatureMatrix,
    groups: BTreeMap<String, Vec<GroupId>>,
}

impl ResearchFrame {
    pub fn new(index: PanelIndex) -> Self {
        let rows = index.len();
        Self {
            index,
            numeric: FeatureMatrix::with_capacity(rows, 0),
            groups: BTreeMap::new(),
        }
    }

    pub fn index(&self) -> &PanelIndex {
        &self.index
    }
    pub fn numeric(&self) -> &FeatureMatrix {
        &self.numeric
    }

    pub fn add_numeric(
        &mut self,
        name: impl Into<String>,
        category: impl Into<String>,
        values: Vec<f64>,
    ) -> ResearchResult<()> {
        if values.len() != self.index.len() {
            return Err(ResearchError::LengthMismatch {
                name: "numeric column".to_string(),
                expected: self.index.len(),
                actual: values.len(),
            });
        }
        self.numeric
            .add_column(Feature::new(name, category, 0), values);
        Ok(())
    }

    pub fn add_group(
        &mut self,
        name: impl Into<String>,
        values: Vec<GroupId>,
    ) -> ResearchResult<()> {
        let name = name.into();
        if values.len() != self.index.len() {
            return Err(ResearchError::LengthMismatch {
                name,
                expected: self.index.len(),
                actual: values.len(),
            });
        }
        self.groups.insert(name, values);
        Ok(())
    }

    pub fn column(&self, name: &str) -> ResearchResult<&[f64]> {
        self.numeric
            .column_by_name(name)
            .ok_or_else(|| ResearchError::MissingColumn(name.to_string()))
    }

    pub fn group(&self, name: &str) -> ResearchResult<&[GroupId]> {
        self.groups
            .get(name)
            .map(Vec::as_slice)
            .ok_or_else(|| ResearchError::MissingGroup(name.to_string()))
    }

    pub fn has_group(&self, name: &str) -> bool {
        self.groups.contains_key(name)
    }
}

/// Borrowed view of one factor column aligned to a research frame.
#[derive(Debug, Clone, Copy)]
pub struct FactorPanelView<'a> {
    pub index: &'a PanelIndex,
    pub factor: &'a [f64],
}

impl<'a> FactorPanelView<'a> {
    pub fn from_frame(frame: &'a ResearchFrame, factor: &str) -> ResearchResult<Self> {
        Ok(Self {
            index: frame.index(),
            factor: frame.column(factor)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_reuses_feature_matrix_and_panel_segments() {
        let index =
            PanelIndex::new(vec![1, 1, 2], vec![AssetId(1), AssetId(2), AssetId(1)]).unwrap();
        let mut frame = ResearchFrame::new(index);
        frame
            .add_numeric("factor", "factor", vec![1.0, 2.0, 3.0])
            .unwrap();
        assert_eq!(frame.numeric().cols(), 1);
        assert_eq!(frame.index().date_segments().offsets(), &[0, 2, 3]);
    }
}
