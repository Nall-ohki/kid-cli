#[derive(Debug, Clone, PartialEq)]
pub enum Color {
    Indexed(u8),         // 256-color palette: \e[48;5;208m
    Rgb(u8, u8, u8),     // True color: \e[48;2;r;g;bm
    Named(u8),           // Basic 16: \e[37m, \e[90m, etc.
}

#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    /// Transparent — terminal default background, no content
    Empty,
    /// A colored "pixel" (Charasay style: 2 spaces with bg color)
    Pixel { bg: Color },
    /// A styled character (CowFiles style: Unicode char with fg/bg)
    Styled {
        ch: char,
        fg: Option<Color>,
        bg: Option<Color>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum CharacterKind {
    /// Cell-based art
    Grid(Vec<Vec<Cell>>),
    /// Raw Sixel bitmap data
    Sixel(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Character {
    pub name: String,
    pub kind: CharacterKind,
    pub height: usize,
    pub width: usize,
}
