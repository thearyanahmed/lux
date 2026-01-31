pub mod brc;

use color_eyre::eyre::Result;

use crate::ui::UI;

pub fn run(helper: &str, rows: u64, measurements: &str, expected: &str) -> Result<()> {
    match helper {
        "1brc" => brc::generate(rows, measurements, expected),
        _ => {
            UI::error(&format!("unknown helper: {}", helper), None);
            UI::note("available helpers: 1brc");
            Ok(())
        }
    }
}
