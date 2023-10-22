use std::{collections::HashSet, env, fmt, fs, path::PathBuf};

use askama::Template;
use geojson::{Feature, FeatureCollection, JsonObject, JsonValue};
use h3o::{geom::ToGeo, CellIndex};

pub struct H3oViewer {
    cells: HashSet<CellIndex>,
    settings: Settings,
}

#[derive(Debug)]
struct Settings {
    cell_labels: bool,
    edge_labels: bool,
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
            cell_labels: true,
            edge_labels: false,
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

    /// Default: on, only works when render_cells_seperately is set (default on)
    pub fn with_cell_labels(mut self, set_on: bool) -> Self {
        self.settings.cell_labels = set_on;
        self
    }

    pub fn with_edge_labels(mut self, set_on: bool) -> Self {
        self.settings.edge_labels = set_on;
        self
    }

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
            let mut feature_list: Vec<Feature> = Vec::new();

            for cell in &self.cells {
                let geometry = cell.to_geojson().unwrap();
                let properties = get_properties(cell);
                let feature = Feature {
                    geometry: Some(geometry),
                    properties: Some(properties),
                    ..Default::default()
                };
                feature_list.push(feature);
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
        if self.settings.cell_labels {
            "var geojson = L.geoJSON(data, {
	        onEachFeature: function (feature, layer) {
            layer.bindTooltip(feature.properties.index, {permanent: true});
        }
    });"
            .to_string()
        } else {
            "var geojson = L.geoJSON(data);".to_string()
        }
    }
}

fn get_properties(cell: &CellIndex) -> JsonObject {
    let mut properties = JsonObject::new();
    properties.insert("index".to_string(), JsonValue::from(cell.to_string()));
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
            .with_cell_labels(true)
            .with_edge_labels(false)
            .show_in_browser();
    }
}
