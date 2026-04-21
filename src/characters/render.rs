use ratatui::style::{Color as TuiColor, Style};
use ratatui::text::{Line, Span};
use crate::characters::types::*;

pub fn render_grid(grid: &[Vec<Cell>]) -> Vec<Line<'static>> {
    grid.iter()
        .map(|row| {
            let spans: Vec<Span<'static>> = row.iter().map(cell_to_span).collect();
            Line::from(spans)
        })
        .collect()
}

pub fn cell_to_span(cell: &Cell) -> Span<'static> {
    match cell {
        Cell::Empty => Span::raw("  "),
        Cell::Pixel { bg } => Span::styled("  ", Style::default().bg(to_ratatui_color(bg))),
        Cell::Styled { ch, fg, bg } => {
            let mut style = Style::default();
            if let Some(fg) = fg {
                style = style.fg(to_ratatui_color(fg));
            }
            if let Some(bg) = bg {
                style = style.bg(to_ratatui_color(bg));
            }
            Span::styled(ch.to_string(), style)
        }
    }
}

pub fn to_ratatui_color(color: &Color) -> TuiColor {
    match color {
        Color::Indexed(n) => TuiColor::Indexed(*n),
        Color::Named(n) => match n {
            30 | 40 => TuiColor::Black,
            31 | 41 => TuiColor::Red,
            32 | 42 => TuiColor::Green,
            33 | 43 => TuiColor::Yellow,
            34 | 44 => TuiColor::Blue,
            35 | 45 => TuiColor::Magenta,
            36 | 46 => TuiColor::Cyan,
            37 | 47 => TuiColor::Gray,
            90 | 100 => TuiColor::DarkGray,
            91 | 101 => TuiColor::LightRed,
            92 | 102 => TuiColor::LightGreen,
            93 | 103 => TuiColor::LightYellow,
            94 | 104 => TuiColor::LightBlue,
            95 | 105 => TuiColor::LightMagenta,
            96 | 106 => TuiColor::LightCyan,
            97 | 107 => TuiColor::White,
            _ => TuiColor::Reset,
        },
        Color::Rgb(r, g, b) => TuiColor::Rgb(*r, *g, *b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_empty_cell() {
        let span = cell_to_span(&Cell::Empty);
        assert_eq!(span.content, "  ");
    }

    #[test]
    fn test_render_pixel_cell() {
        let cell = Cell::Pixel { bg: Color::Indexed(208) };
        let span = cell_to_span(&cell);
        assert_eq!(span.style.bg, Some(TuiColor::Indexed(208)));
    }

    #[test]
    fn test_render_grid_line_count() {
        let grid = vec![
            vec![Cell::Empty],
            vec![Cell::Empty, Cell::Empty],
        ];
        let lines = render_grid(&grid);
        assert_eq!(lines.len(), 2);
    }
}
