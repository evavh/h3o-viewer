use std::{
    collections::HashSet,
    env, fmt, fs,
    hash::{DefaultHasher, Hash, Hasher},
    path::PathBuf,
};

use geojson::{Feature, FeatureCollection, JsonObject, JsonValue};
use h3o::{geom::ToGeo, CellIndex, DirectedEdgeIndex, LatLng};

pub struct H3oViewer {
    cell_groups: Vec<Vec<CellIndex>>,
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
    filename: Option<String>,
}

const HTML_TEMPLATE: &str = include_str!("../templates/viewer.html");

impl fmt::Debug for H3oViewer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.debug_struct("H3oViewer")
            .field("cells", &"Iterator over CellIndexes")
            .field("settings", &self.settings)
            .field("circles", &self.circles)
            .finish()
    }
}

impl Hash for H3oViewer {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        format!("{:?}", self.cell_groups).hash(state);
        format!("{:?}", self.settings).hash(state);
        format!("{:?}", self.circles).hash(state);
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            cell_resolutions: false,
            cell_indexes: false,
            edge_lengths: false,
            separate_cells: true,
            filename: None,
        }
    }
}
impl H3oViewer {
    pub fn for_cells(cells: impl IntoIterator<Item = CellIndex>) -> Self {
        H3oViewer {
            cell_groups: Vec::from([cells.into_iter().collect()]),
            settings: Settings::default(),
            circles: Vec::new(),
        }
    }

    pub fn for_cell_groups(
        cells: impl IntoIterator<Item = impl IntoIterator<Item = CellIndex>>,
    ) -> Self {
        H3oViewer {
            cell_groups: cells
                .into_iter()
                .map(|cells| cells.into_iter().collect())
                .collect(),
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

    pub fn custom_filename(mut self, filename: &str) -> Self {
        self.settings.filename = Some(filename.to_owned());
        self
    }

    #[must_use]
    pub fn draw_circle(mut self, center: LatLng, radius: usize) -> Self {
        self.circles.push((center, radius));
        self
    }

    pub fn show_in_browser(self) {
        let mut state = DefaultHasher::new();
        self.hash(&mut state);
        let hash = state.finish();

        let filename = match self.settings.filename.clone() {
            Some(override_filename) => override_filename,
            None => format!("h3o-viewer-{hash:X?}.html"),
        };
        let html = self.generate_html();
        let _ = open_in_browser(&html, &filename);
    }

    #[must_use]
    pub fn generate_html(self) -> String {
        let geometry = self.cells_to_features();
        let geojson = geometry.to_string();
        let circles = self.generate_circles();

        HTML_TEMPLATE
            .replace("{{geojson}}", &geojson)
            .replace("{{circles}}", &circles)
    }

    fn cells_to_features(&self) -> FeatureCollection {
        if self.settings.separate_cells && self.cell_groups.len() == 1 {
            let mut feature_list = Vec::new();
            let mut edges_seen = Vec::new();

            for cell in &self.cell_groups[0] {
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
            self.cell_groups
                .iter()
                .map(|cell_group| {
                    let mut cell_group = cell_group.clone();
                    cell_group.sort();
                    cell_group.dedup();

                    let geometry = cell_group
                        .clone()
                        .to_geojson()
                        .expect("Resolution should be homogenous, and no duplicate cells");
                    Feature {
                        geometry: Some(geometry),
                        ..Default::default()
                    }
                })
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

fn open_in_browser(html: &str, filename: &str) -> Result<(), std::io::Error> {
    let cargo_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let default_path: PathBuf =
        [&cargo_dir, "target", filename].iter().collect();
    let second_path: PathBuf = [&cargo_dir, filename].iter().collect();
    #[allow(clippy::single_match_else)]
    let path = match fs::write(&default_path, html) {
        Ok(()) => default_path,
        Err(_) => {
            fs::write(&second_path, html)?;
            second_path
        }
    };

    webbrowser::open(&path.into_os_string().into_string().unwrap())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use h3o::Resolution;

    use super::*;

    #[test]
    fn opens_in_browser() {
        let center_cell = CellIndex::try_from(0x8a1fb46622dffff).unwrap();
        let mut cells = center_cell.grid_disk::<Vec<_>>(1);
        cells.push(cells[0].parent(Resolution::Nine).unwrap());

        dbg!(H3oViewer::for_cells(cells)
            .with_cell_resolutions(false)
            .with_edge_lengths(true))
        .draw_circle(center_cell.into(), 150)
        .draw_circle(center_cell.into(), 200)
        .show_in_browser();
    }
}
