use std::{
    collections::HashSet,
    env, fmt,
    fs::{self, File},
    path::PathBuf,
};

use askama::Template;
use geojson::{Geometry, Value::GeometryCollection};
use h3o::{geom::ToGeo, CellIndex, LatLng};

struct H3oViewer {
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
struct HtmlTemplate<'a> {
    geojson: &'a str,
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
        let geometry: Geometry = if self.settings.separate_cells {
            let geometry_collection: Vec<_> = self
                .cells
                .into_iter()
                .map(CellIndex::to_geojson)
                .map(Result::unwrap)
                .collect();
            GeometryCollection(geometry_collection).into()
        } else {
            self.cells.to_geojson().unwrap()
        };
        let template = HtmlTemplate {
            geojson: &geometry.to_string(),
        };
        template.render().unwrap()
    }
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
    fn correct_html() {
        let cells = [CellIndex::try_from(0x8a1fb46622dffff).unwrap()];

        let html = H3oViewer::for_cells(cells[0].grid_disk::<Vec<_>>(1))
            .with_cell_labels(true)
            .with_edge_labels(false)
            .generate_html();
        assert_eq!(html, "");
    }

    #[test]
    fn opens_in_browser() {
        let cells = [CellIndex::try_from(0x8a1fb46622dffff).unwrap()];

        H3oViewer::for_cells(cells[0].grid_disk::<Vec<_>>(1))
            .with_cell_labels(true)
            .with_edge_labels(false)
            .show_in_browser();
    }
}
