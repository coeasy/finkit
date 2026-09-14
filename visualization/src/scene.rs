use crate::geometry::{Point, Rect};

/// A semantic panel in the chart scene.
#[derive(Debug, Clone, PartialEq)]
pub struct PanelDescriptor {
    pub id: PanelId,
    pub rect: Rect,
    pub visible: bool,
}

/// Source and timeframe metadata exported with the scene.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChartMetadata {
    pub data_revision: u64,
    pub source_start: usize,
    pub source_end: usize,
    pub timeframe_labels: Vec<String>,
}

/// Semantic chart panel identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelId {
    Main,
    Volume,
    Indicator(usize),
}

/// Metadata for a renderable chart layer.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerDescriptor {
    pub id: String,
    pub panel: PanelId,
    pub z_index: i32,
    pub visible: bool,
    pub opacity: f32,
}

impl LayerDescriptor {
    pub fn new(id: impl Into<String>, panel: PanelId, z_index: i32) -> Self {
        Self {
            id: id.into(),
            panel,
            z_index,
            visible: true,
            opacity: 1.0,
        }
    }
}

/// Semantic target returned by hit testing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HitTarget {
    Kline {
        index: usize,
    },
    Volume {
        index: usize,
    },
    Indicator {
        name: String,
        index: usize,
    },
    ChanFractal {
        index: usize,
    },
    ChanStroke {
        start_index: usize,
        end_index: usize,
    },
    ChanSegment {
        start_index: usize,
        end_index: usize,
    },
    ChanCenter {
        start_index: usize,
        end_index: usize,
        level: usize,
    },
    ChanSignal {
        index: usize,
        kind: String,
    },
    ChanDivergence {
        index: usize,
        kind: String,
    },
    Event {
        index: usize,
        kind: String,
    },
}

/// A lightweight spatial hit region independent from DrawList primitives.
#[derive(Debug, Clone, PartialEq)]
pub struct HitRegion {
    pub rect: Rect,
    pub target: HitTarget,
    pub priority: i32,
    pub tooltip: Option<String>,
}

impl HitRegion {
    pub fn contains(&self, point: Point) -> bool {
        self.rect.contains(&point)
    }
}

/// Semantic scene metadata shared by SVG, HTML, native and WASM frontends.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChartScene {
    pub panels: Vec<PanelDescriptor>,
    pub layers: Vec<LayerDescriptor>,
    pub hit_regions: Vec<HitRegion>,
    pub metadata: ChartMetadata,
}

impl ChartScene {
    pub fn clear(&mut self) {
        self.panels.clear();
        self.layers.clear();
        self.hit_regions.clear();
        self.metadata = ChartMetadata::default();
    }

    pub fn add_panel(&mut self, panel: PanelDescriptor) {
        self.panels.push(panel);
    }

    pub fn add_layer(&mut self, layer: LayerDescriptor) {
        self.layers.push(layer);
        self.layers.sort_by_key(|layer| layer.z_index);
    }

    pub fn set_layer_visible(&mut self, id: &str, visible: bool) -> bool {
        if let Some(layer) = self.layers.iter_mut().find(|layer| layer.id == id) {
            layer.visible = visible;
            true
        } else {
            false
        }
    }

    pub fn is_layer_visible(&self, id: &str) -> bool {
        self.layers
            .iter()
            .find(|layer| layer.id == id)
            .map(|layer| layer.visible)
            .unwrap_or(true)
    }

    pub fn add_hit_region(&mut self, region: HitRegion) {
        self.hit_regions.push(region);
    }

    pub fn hit_test(&self, point: Point) -> Option<&HitRegion> {
        self.hit_regions
            .iter()
            .filter(|region| region.contains(point))
            .max_by_key(|region| region.priority)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_sorts_layers_and_controls_visibility() {
        let mut scene = ChartScene::default();
        scene.add_layer(LayerDescriptor::new("signals", PanelId::Main, 20));
        scene.add_layer(LayerDescriptor::new("grid", PanelId::Main, 0));
        assert_eq!(scene.layers[0].id, "grid");
        assert!(scene.set_layer_visible("signals", false));
        assert!(!scene.is_layer_visible("signals"));
    }

    #[test]
    fn hit_test_prefers_highest_priority() {
        let mut scene = ChartScene::default();
        scene.add_hit_region(HitRegion {
            rect: Rect::new(0.0, 0.0, 20.0, 20.0),
            target: HitTarget::Kline { index: 1 },
            priority: 1,
            tooltip: None,
        });
        scene.add_hit_region(HitRegion {
            rect: Rect::new(5.0, 5.0, 10.0, 10.0),
            target: HitTarget::ChanSignal {
                index: 1,
                kind: "B1".into(),
            },
            priority: 10,
            tooltip: Some("B1".into()),
        });
        assert!(matches!(
            scene
                .hit_test(Point::new(10.0, 10.0))
                .map(|region| &region.target),
            Some(HitTarget::ChanSignal { .. })
        ));
    }
}
