#![cfg(test)]
mod parser_core;

use crate::characters::types::*;
use self::parser_core as core;

pub fn parse(id: &str, content: &str, source: Source) -> anyhow::Result<Character> {
    let core_source = match source {
        Source::CowFiles => core::Source::CowFiles,
        Source::Charasay | Source::User => core::Source::Charasay,
    };

    let core_chara = core::parse(id, content, core_source)?;
    
    Ok(Character {
        id: core_chara.id,
        name: core_chara.name,
        source,
        kind: map_kind(core_chara.kind),
        height: core_chara.height,
        width: core_chara.width,
    })
}

fn map_kind(kind: core::CharacterKind) -> CharacterKind {
    match kind {
        core::CharacterKind::Sixel(s) => CharacterKind::Sixel(s),
        core::CharacterKind::Grid(grid) => CharacterKind::Grid(
            grid.into_iter().map(|row| row.into_iter().map(map_cell).collect()).collect()
        ),
    }
}

fn map_cell(cell: core::Cell) -> Cell {
    match cell {
        core::Cell::Connector { ch } => Cell::Connector { ch },
        core::Cell::Styled { ch, fg, bg } => Cell::Styled {
            ch,
            fg: fg.map(map_color),
            bg: bg.map(map_color),
        },
    }
}

fn map_color(color: core::Color) -> Color {
    match color {
        core::Color::Indexed(n) => Color::Indexed(n),
        core::Color::Named(n) => Color::Named(n),
        core::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_comments() {
        let content = "# comment\n$the_cow = <<EOC\nline1\n## comment\nline2\nEOC";
        let chara = parse("test", content, Source::CowFiles).unwrap();
        if let CharacterKind::Grid(grid) = chara.kind {
            assert_eq!(grid.len(), 2);
        }
    }

    #[test]
    fn test_sixel_detection() {
        let content = "$the_cow = <<EOC\n\\x1BP0;1q...\nEOC";
        let chara = parse("test", content, Source::CowFiles).unwrap();
        assert!(matches!(chara.kind, CharacterKind::Sixel(_)));
    }

    #[test]
    fn test_thoughts_replacement() {
        let content = "$the_cow = <<EOC\n$thoughts\nEOC";
        let chara = parse("test", content, Source::Charasay).unwrap();
        if let CharacterKind::Grid(grid) = chara.kind {
            assert_eq!(grid[0][0], Cell::Connector { ch: '\\' });
            assert_eq!(grid[0][1], Cell::Styled { ch: ' ', fg: None, bg: None });
        }
    }
}
