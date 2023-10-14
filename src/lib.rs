use h3o::CellIndex;

struct H3oViewer {
    html: String,
    settings: Settings,
}

struct Settings {
    cell_labels: bool,
    edge_labels: bool,
}

impl H3oViewer {
    pub fn new(
        cells: impl IntoIterator<Item = CellIndex>,
        settings: Settings,
    ) -> Self {
        let html = H3oViewer::generate_html(cells);
        H3oViewer { html, settings }
    }

    pub fn show(&self) {
        open_in_browser(&self.html);
    }

    fn generate_html(cells: impl IntoIterator<Item = CellIndex>) -> String {
        todo!()
    }
}

fn open_in_browser(html: &str) {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correct_html() {
        let cells = [CellIndex::try_from(0x8a1fb46622dffff).unwrap()];
        let settings = Settings { cell_labels: false, edge_labels: true };

        let viewer = H3oViewer::new(cells, settings);
        let html = viewer.html;
        assert_eq!(html, "");
    }
}
