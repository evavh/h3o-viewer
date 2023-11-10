use std::{
    collections::HashSet, env, error::Error, fmt, fs, io::Error, path::PathBuf,
};

use askama::Template;
use geojson::{Feature, FeatureCollection, JsonObject, JsonValue};
use h3o::{geom::ToGeo, CellIndex, DirectedEdgeIndex, LatLng};

pub struct H3oViewer {
    cells: HashSet<CellIndex>,
    settings: Settings,
    circles: Vec<(LatLng, usize)>,
}

#[derive(Debug)]
#[allow(clippy::struct_excessive_bools)] // setter functions make this ok
struct Settings {
    cell_indexes: bool,
    edge_lengths: bool,
    separate_cells: bool,
    cell_resolutions: bool,
}

#[derive(Template)]
#[template(path = "viewer.html")]
struct HtmlTemplate {
    geojson: String,
    circles: String,
}

impl fmt::Debug for H3oViewer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.debug_struct("H3oViewer")
            .field("cells", &"Iterator over CellIndexes")
            .field("settings", &self.settings)
            .field("circles", &self.circles)
            .finish()
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            cell_resolutions: true,
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
            circles: Vec::new(),
        }
    }

    /// Default: off, only works when `render_cells_seperately` is set (default on)
    #[must_use]
    pub fn with_cell_indexes(mut self, set_on: bool) -> Self {
        self.settings.cell_indexes = set_on;
        self
    }

    /// Default: off, only works when `render_cells_seperately` is set (default
    /// on)
    #[must_use]
    pub fn with_edge_lengths(mut self, set_on: bool) -> Self {
        self.settings.edge_lengths = set_on;
        self
    }

    /// Default: on, only works when `render_cells_seperately` is set (default on)
    #[must_use]
    pub fn with_cell_resolutions(mut self, set_on: bool) -> Self {
        self.settings.cell_resolutions = set_on;
        self
    }

    /// Default: on, recommended to turn off if rendering is very slow for
    /// a large number of cells
    #[must_use]
    pub fn render_cells_separately(mut self, set_on: bool) -> Self {
        self.settings.separate_cells = set_on;
        self
    }

    #[must_use]
    pub fn draw_circle(mut self, center: LatLng, radius: usize) -> Self {
        self.circles.push((center, radius));
        self
    }

    pub fn show_in_browser(self) {
        let html = self.generate_html();
        open_in_browser(&html);
    }

    #[must_use]
    pub fn generate_html(self) -> String {
        let geometry = self.cells_to_features();
        let template = HtmlTemplate {
            geojson: geometry.to_string(),
            circles: self.generate_circles(),
        };
        template
            .render()
            .expect("Writing strings into strings should not fail")
    }

    fn cells_to_features(&self) -> FeatureCollection {
        if self.settings.separate_cells {
            let mut feature_list = Vec::new();
            let mut edges_seen = Vec::new();

            for cell in &self.cells {
                let cell_feature = self.cell_to_feature(*cell);
                feature_list.push(cell_feature);

                if self.settings.edge_lengths {
                    for edge in cell.edges() {
                        if !edges_seen.contains(&inverse(edge)) {
                            let edge_feature = Self::edge_to_feature(edge);
                            feature_list.push(edge_feature);
                            edges_seen.push(edge.cells());
                        }
                    }
                }
            }
            feature_list.into_iter().collect()
        } else {
            let geometry = self
                .cells
                .clone()
                .to_geojson()
                .expect("Cannot fail because to_geom cannot fail");
            [Feature {
                geometry: Some(geometry),
                ..Default::default()
            }]
            .into_iter()
            .collect()
        }
    }

    fn cell_to_feature(&self, cell: CellIndex) -> Feature {
        let geometry = cell
            .to_geojson()
            .expect("Cannot fail because to_geom cannot fail");
        let properties = self.get_cell_properties(cell);

        Feature {
            geometry: Some(geometry),
            properties: Some(properties),
            ..Default::default()
        }
    }

    fn edge_to_feature(edge: DirectedEdgeIndex) -> Feature {
        let geometry = edge
            .to_geojson()
            .expect("Cannot fail because to_geom cannot fail");
        let properties = Self::get_edge_properties(edge);

        Feature {
            geometry: Some(geometry),
            properties: Some(properties),
            ..Default::default()
        }
    }

    fn get_cell_properties(&self, cell: CellIndex) -> JsonObject {
        let mut properties = JsonObject::new();
        let mut val = String::new();

        if self.settings.cell_resolutions {
            val += &format!("Res: {}", cell.resolution());
        }

        if self.settings.cell_resolutions && self.settings.cell_indexes {
            val += "<br>";
        }

        if self.settings.cell_indexes {
            val += &cell.to_string();
        }

        properties.insert("label".to_string(), JsonValue::from(val));
        properties
    }

    fn get_edge_properties(edge: DirectedEdgeIndex) -> JsonObject {
        let mut properties = JsonObject::new();
        let length = if edge.length_m() > 1000.0 {
            format!("{:.0} km", edge.length_km())
        } else {
            format!("{:.0} m", edge.length_m())
        };
        properties.insert("label".to_string(), JsonValue::from(length));
        properties
    }

    fn generate_circles(&self) -> String {
        // clippy version is very unclear, perf doesn't matter here
        #[allow(clippy::format_collect)]
        self.circles
            .iter()
            .map(|(c, r)| {
                format!(
                    "L.circle([{}, {}], {{radius: {r}, fill: false, color: '#ee0000'}}).addTo(map);\n",
                    c.lat(),
                    c.lng()
                )
            })
            .collect()
    }
}

fn inverse(edge: DirectedEdgeIndex) -> (CellIndex, CellIndex) {
    (edge.destination(), edge.origin())
}

fn open_in_browser(html: &str) -> Result<(), std::io::Error> {
    let cargo_dir = env::var("CARGO_MANIFEST_DIR")?;
    let default_path: PathBuf =
        [&cargo_dir, "target", "h3o-viewer.html"].iter().collect();
    let second_path: PathBuf = [&cargo_dir, "h3o-viewer.html"].iter().collect();
    #[allow(clippy::single_match_else)]
    let path = match fs::write(&default_path, html) {
        Ok(()) => default_path,
        Err(_) => {
            fs::write(&second_path, html)?;
            second_path
        }
    };

    webbrowser::open(&path.into_os_string().into_string()?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_in_browser() {
        let cells = [CellIndex::try_from(0x8a1fb46622dffff).unwrap()];

        dbg!(H3oViewer::for_cells(cells[0].grid_disk::<Vec<_>>(1))
            .with_cell_resolutions(false)
            .with_edge_lengths(true))
        .draw_circle(cells[0].into(), 150)
        .draw_circle(cells[0].into(), 200)
        .show_in_browser();
    }
}
