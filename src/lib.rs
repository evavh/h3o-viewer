use std::{collections::HashSet, env, fmt, fs, path::PathBuf};

use askama::Template;
use geojson::{Feature, FeatureCollection, JsonObject, JsonValue};
use h3o::{geom::ToGeo, CellIndex, DirectedEdgeIndex};

pub struct H3oViewer {
    cells: HashSet<CellIndex>,
    settings: Settings,
}

#[derive(Debug)]
struct Settings {
    cell_indexes: bool,
    edge_lengths: bool,
    separate_cells: bool,
}

#[derive(Template)]
#[template(path = "viewer.html")]
struct HtmlTemplate {
    geojson: String,
    geometry_code: String,
}

impl fmt::Debug for H3oViewer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.debug_struct("H3oViewer")
            .field("cells", &"Iterator over CellIndexes")
            .field("settings", &self.settings)
            .finish()
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            cell_indexes: false,
            edge_lengths: false,
            separate_cells: true,
        }
    }
}
impl H3oViewer {
    pub fn for_cells(cells: impl IntoIterator<Item = CellIndex>) -> Self {
        H3oViewer {
            cells: cells.into_iter().collect(),
            settings: Settings::default(),
        }
    }

    /// Default: off, only works when render_cells_seperately is set (default on)
    pub fn with_cell_indexes(mut self, set_on: bool) -> Self {
        self.settings.cell_indexes = set_on;
        self
    }

    /// Default: off, only works when render_cells_seperately is set (default
    /// on)
    pub fn with_edge_lengths(mut self, set_on: bool) -> Self {
        self.settings.edge_lengths = set_on;
        self
    }

    /// Default: on, recommended to turn off if rendering is very slow for
    /// a large number of cells
    pub fn render_cells_separately(mut self, set_on: bool) -> Self {
        self.settings.separate_cells = set_on;
        self
    }

    pub fn show_in_browser(self) {
        let html = self.generate_html();
        open_in_browser(&html);
    }

    pub fn generate_html(self) -> String {
        let geometry = self.cells_to_features();
        let template = HtmlTemplate {
            geojson: geometry.to_string(),
            geometry_code: self.pick_geometry_code(),
        };
        template.render().unwrap()
    }

    fn cells_to_features(&self) -> FeatureCollection {
        if self.settings.separate_cells {
            let mut feature_list = Vec::new();

            for cell in &self.cells {
                let cell_feature = cell_to_feature(cell);
                feature_list.push(cell_feature);

                if self.settings.edge_lengths {
                    for edge in cell.edges() {
                        let edge_feature = edge_to_feature(edge);
                        feature_list.push(edge_feature);
                    }
                }
            }
            feature_list.into_iter().collect()
        } else {
            let geometry = self.cells.clone().to_geojson().unwrap();
            [Feature {
                geometry: Some(geometry),
                ..Default::default()
            }]
            .into_iter()
            .collect()
        }
    }

    fn pick_geometry_code(&self) -> String {
        if self.settings.cell_indexes {
            "var geojson = L.geoJSON(data, {
	        onEachFeature: function (feature, layer) {
            layer.bindTooltip(feature.properties.label, {permanent: true});
        }
    });"
            .to_string()
        } else {
            "var geojson = L.geoJSON(data);".to_string()
        }
    }
}

fn cell_to_feature(cell: &CellIndex) -> Feature {
    let geometry = cell.to_geojson().unwrap();
    let properties = get_cell_properties(cell);
    let cell_feature = Feature {
        geometry: Some(geometry),
        properties: Some(properties),
        ..Default::default()
    };
    cell_feature
}

fn edge_to_feature(edge: DirectedEdgeIndex) -> Feature {
    let geometry = edge.to_geojson().unwrap();
    let properties = get_edge_properties(&edge);
    let edge_feature = Feature {
        geometry: Some(geometry),
        properties: Some(properties),
        ..Default::default()
    };
    edge_feature
}

fn get_cell_properties(cell: &CellIndex) -> JsonObject {
    let mut properties = JsonObject::new();
    properties.insert("label".to_string(), JsonValue::from(cell.to_string()));
    properties
}

fn get_edge_properties(edge: &DirectedEdgeIndex) -> JsonObject {
    let mut properties = JsonObject::new();
    let length = if edge.length_m() > 1000.0 {
        format!("{:.0} km", edge.length_km())
    } else {
        format!("{:.0} m", edge.length_m())
    };
    properties.insert("label".to_string(), JsonValue::from(length));
    properties
}

fn open_in_browser(html: &str) {
    let cargo_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let path: PathBuf =
        [&cargo_dir, "target", "h3o-viewer.html"].iter().collect();
    fs::write(&path, html).unwrap();

    webbrowser::open(&path.into_os_string().into_string().unwrap()).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_in_browser() {
        let cells = [CellIndex::try_from(0x8a1fb46622dffff).unwrap()];

        H3oViewer::for_cells(cells[0].grid_disk::<Vec<_>>(1))
            .with_cell_indexes(true)
            .with_edge_lengths(true)
            .show_in_browser();
    }
}
