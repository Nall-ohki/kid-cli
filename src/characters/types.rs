#[derive(Debug, Clone, PartialEq)]
pub enum Color {
    Indexed(u8),         // 256-color palette: \e[48;5;208m
    Rgb(u8, u8, u8),     // True color: \e[48;2;r;g;bm
    Named(u8),           // Basic 16: \e[37m, \e[90m, etc.
}

#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    /// A styled character belonging to the character art
    Styled {
        ch: char,
        fg: Option<Color>,
        bg: Option<Color>,
    },
    /// A speech or thought bubble connector
    Connector {
        ch: char,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum CharacterKind {
    /// Cell-based art
    Grid(Vec<Vec<Cell>>),
    /// Raw Sixel bitmap data
    Sixel(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Charasay,
    CowFiles,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Character {
    pub id: String,          // The filename/internal ID
    pub name: String,        // Human-readable name
    pub source: Source,      // Origin of the asset
    pub kind: CharacterKind,
    pub height: usize,
    pub width: usize,
}
