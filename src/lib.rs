use h3o::CellIndex;

pub struct Settings {
    pub cell_labels: bool,
    pub edge_labels: bool,
}

pub fn show_in_browser(
    cells: impl IntoIterator<Item = CellIndex>,
    settings: Settings,
) {
    let html = generate_html(cells, settings);
    open_in_browser(&html);
}

fn generate_html(
    cells: impl IntoIterator<Item = CellIndex>,
    settings: Settings,
) -> String {
    todo!()
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
        let settings = Settings {
            cell_labels: false,
            edge_labels: true,
        };

        let html = generate_html(cells, settings);
        assert_eq!(html, "");
    }
}
