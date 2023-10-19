use std::{
    env, fmt,
    fs::{self, File},
};

use h3o::CellIndex;
use askama::Template;

struct H3oViewer<I: Iterator<Item = CellIndex>> {
    cells: I,
    settings: Settings,
}

#[derive(Debug)]
struct Settings {
    cell_labels: bool,
    edge_labels: bool,
}

#[derive(Template)]
#[template(path = "viewer.html")]
struct HtmlTemplate {
    geojson: String,
}

impl<I: Iterator<Item = CellIndex>> fmt::Debug for H3oViewer<I> {
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
        }
    }
}
impl<I: Iterator<Item = CellIndex>> H3oViewer<I> {
    pub fn for_cells<
        T: IntoIterator<Item = CellIndex, IntoIter = I> + Clone,
    >(
        cells: T,
    ) -> Self {
        H3oViewer {
            cells: cells.into_iter(),
            settings: Settings::default(),
        }
    }

    pub fn with_cell_labels(&mut self) -> &mut Self {
        self.settings.cell_labels = true;
        self
    }
    pub fn without_cell_labels(&mut self) -> &mut Self {
        self.settings.cell_labels = false;
        self
    }
    pub fn with_edge_labels(&mut self) -> &mut Self {
        self.settings.edge_labels = true;
        self
    }
    pub fn without_edge_labels(&mut self) -> &mut Self {
        self.settings.edge_labels = false;
        self
    }

    pub fn show_in_browser(&self) {
        let html = self.generate_html();
        open_in_browser(&html);
    }

    pub fn generate_html(&self) -> String {
        let geojson = todo!();
        let template = HtmlTemplate { geojson };
        template.render().unwrap()
    }
}

fn open_in_browser(html: &str) {
    let target_dir: &str = &env::var("CARGO_TARGET_DIR").unwrap();
    let path = format!("{target_dir}h3o-viewer.html");
    fs::write(&path, html).unwrap();

    webbrowser::open(&path).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correct_html() {
        let cells = [CellIndex::try_from(0x8a1fb46622dffff).unwrap()];

        let html = H3oViewer::for_cells(cells)
            .with_cell_labels()
            .without_edge_labels()
            .generate_html();
        assert_eq!(html, "");
    }

    #[test]
    fn opens_in_browser() {
        let cells = [CellIndex::try_from(0x8a1fb46622dffff).unwrap()];

        H3oViewer::for_cells(cells)
            .with_cell_labels()
            .without_edge_labels()
            .show_in_browser();
    }
}
