use std::env;
use std::fs;
use std::path::Path;
use regex::Regex;
use std::collections::HashMap;

// --- Simplified Parser for build.rs ---
// We need to keep this in sync with src/characters/parser.rs or find a way to share.
// For now, I'll implement a slightly more robust way to share by including the file but mocking the module.

fn main() -> anyhow::Result<()> {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("builtins.rs");
    
    let mut builtins = Vec::new();
    let asset_dir = Path::new("assets/characters");
    
    if asset_dir.exists() {
        for entry in fs::read_dir(asset_dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "chara" || ext == "cow" {
                    let name = path.file_stem().unwrap().to_string_lossy().to_string();
                    let content = fs::read_to_string(&path)?;
                    builtins.push((name, content));
                }
            }
        }
    }
    
    println!("cargo:warning=Baked {} characters into binary", builtins.len());

    let mut code = String::new();
    code.push_str("use crate::characters::types::*;\n\n");
    code.push_str("pub fn load_builtins() -> Vec<Character> {\n");
    code.push_str("    vec![\n");

    for (name, content) in builtins {
        // We'll parse here using a local version of the parser
        // Since we want Phase 2 to work, I'll implement the parsing logic here or include it.
        // Let's use a trick: define the parser in a way it can be shared.
        // For build.rs, I'll just implement a generator that produces the vec![...] code.
        
        // Wait, if I parse at build time, I need to generate the recursive Vec<Vec<Cell>> structure as code.
        // This is exactly what the user wanted: "compile-time conversion".
        
        if let Ok(chara) = parse_local(&name, &content) {
            code.push_str(&format!("        Character {{\n"));
            code.push_str(&format!("            name: {:?}.to_string(),\n", chara.name));
            code.push_str(&format!("            kind: {},\n", kind_to_code(&chara.kind)));
            code.push_str(&format!("            height: {},\n", chara.height));
            code.push_str(&format!("            width: {},\n", chara.width));
            code.push_str("        },\n");
        }
    }

    code.push_str("    ]\n");
    code.push_str("}\n");

    fs::write(&dest_path, code)?;
    println!("cargo:rerun-if-changed=assets/characters");
    
    Ok(())
}

// Minimal types for generator
#[derive(Debug, Clone, PartialEq)]
pub enum Color {
    Indexed(u8),
    Rgb(u8, u8, u8),
    Named(u8),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    Empty,
    Pixel { bg: Color },
    Styled { ch: char, fg: Option<Color>, bg: Option<Color> },
}

#[derive(Debug, Clone, PartialEq)]
pub enum CharacterKind {
    Grid(Vec<Vec<Cell>>),
    Sixel(String),
}

pub struct Character {
    pub name: String,
    pub kind: CharacterKind,
    pub height: usize,
    pub width: usize,
}

fn kind_to_code(kind: &CharacterKind) -> String {
    match kind {
        CharacterKind::Sixel(s) => format!("CharacterKind::Sixel({:?}.to_string())", s),
        CharacterKind::Grid(grid) => {
            let mut s = String::from("CharacterKind::Grid(vec![\n");
            for row in grid {
                s.push_str("                vec![");
                for cell in row {
                    s.push_str(&cell_to_code(cell));
                    s.push_str(", ");
                }
                s.push_str("],\n");
            }
            s.push_str("            ])");
            s.to_string()
        }
    }
}

fn cell_to_code(cell: &Cell) -> String {
    match cell {
        Cell::Empty => "Cell::Empty".to_string(),
        Cell::Pixel { bg } => format!("Cell::Pixel {{ bg: {} }}", color_to_code(bg)),
        Cell::Styled { ch, fg, bg } => {
            let fg_code = fg.as_ref().map_or("None".to_string(), |c| format!("Some({})", color_to_code(c)));
            let bg_code = bg.as_ref().map_or("None".to_string(), |c| format!("Some({})", color_to_code(c)));
            format!("Cell::Styled {{ ch: {:?}, fg: {}, bg: {} }}", ch, fg_code, bg_code)
        }
    }
}

fn color_to_code(color: &Color) -> String {
    match color {
        Color::Indexed(n) => format!("Color::Indexed({})", n),
        Color::Named(n) => format!("Color::Named({})", n),
        Color::Rgb(r, g, b) => format!("Color::Rgb({}, {}, {})", r, g, b),
    }
}

// --- Copy of parser logic (simplified/standalone) ---

fn parse_local(name: &str, content: &str) -> anyhow::Result<Character> {
    let lines: Vec<&str> = content.lines().collect();
    let mut vars = HashMap::new();
    let mut body_lines = Vec::new();
    let mut in_body = false;
    let mut body_marker = String::new();

    let var_re = Regex::new(r#"^\s*(\$\w+)\s*=\s*("[^"]*"|'[^']*'|[^\#;]+)"#)?;
    let body_start_re = Regex::new(r"^\s*(\$\w+)\s*=\s*<<(\w+);?")?;

    for line in &lines {
        let trimmed = line.trim();
        if trimmed.starts_with('#') { continue; }
        if !in_body {
            if let Some(caps) = body_start_re.captures(line) {
                in_body = true;
                body_marker = caps.get(2).unwrap().as_str().to_string();
                continue;
            }
            if let Some(caps) = var_re.captures(line) {
                let key = caps.get(1).unwrap().as_str();
                let val = caps.get(2).unwrap().as_str().trim().trim_matches('"').trim_matches('\'');
                vars.insert(key.to_string(), val.to_string());
                continue;
            }
        } else {
            if trimmed == body_marker || trimmed == format!("{};", body_marker) {
                break;
            }
            body_lines.push(*line);
        }
    }

    let full_body = body_lines.join("\n");
    if full_body.contains("\x1BP") || full_body.contains("\\x1BP") {
        let mut width = 0;
        let mut height = 0;
        let sixel_re = Regex::new(r#"\x1BP[0-9;]*q"1;1;(\d+);(\d+)"#)?;
        if let Some(caps) = sixel_re.captures(&full_body.replace("\\x1B", "\x1B")) {
            width = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
            height = caps.get(2).unwrap().as_str().parse().unwrap_or(0);
        }

         return Ok(Character {
            name: name.to_string(),
            kind: CharacterKind::Sixel(full_body),
            height: if height > 0 { (height / 10) as usize } else { body_lines.len() },
            width: if width > 0 { (width / 5) as usize } else { 0 },
        });
    }

    let mut resolved_lines = Vec::new();
    for line in body_lines {
        let mut resolved = line.to_string();
        let mut sorted_vars: Vec<_> = vars.iter().collect();
        sorted_vars.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        for (key, val) in sorted_vars {
            resolved = resolved.replace(key, val);
        }
        resolved = resolved.replace("$thoughts", "\\ ");
        resolved = resolved.replace("$t", "\\ ");
        resolved = resolved.replace("\\e", "\x1B");
        resolved = normalize_unicode_escapes(&resolved);
        resolved = normalize_hex_escapes(&resolved);
        resolved_lines.push(resolved);
    }

    let grid = construct_grid(&resolved_lines);
    let height = grid.len();
    let width = grid.iter().map(|r| r.len()).max().unwrap_or(0);

    Ok(Character {
        name: name.to_string(),
        kind: CharacterKind::Grid(grid),
        height,
        width,
    })
}

fn normalize_hex_escapes(s: &str) -> String {
    let re = Regex::new(r"\\x([0-9A-Fa-f]{2})").unwrap();
    re.replace_all(s, |caps: &regex::Captures| {
        let hex = caps.get(1).unwrap().as_str();
        if let Ok(u) = u8::from_str_radix(hex, 16) {
            return (u as char).to_string();
        }
        caps.get(0).unwrap().as_str().to_string()
    }).to_string()
}

fn normalize_unicode_escapes(s: &str) -> String {
    let re = Regex::new(r"\\x\{([0-9A-Fa-f]+)\}").unwrap();
    re.replace_all(s, |caps: &regex::Captures| {
        let hex = caps.get(1).unwrap().as_str();
        if let Ok(u) = u32::from_str_radix(hex, 16) {
            if let Some(c) = std::char::from_u32(u) {
                return c.to_string();
            }
        }
        caps.get(0).unwrap().as_str().to_string()
    }).to_string()
}

fn construct_grid(lines: &[String]) -> Vec<Vec<Cell>> {
    let mut grid = Vec::new();
    let ansi_re = Regex::new(r"\x1B\[([0-9;]*)m").unwrap();
    for line in lines {
        let mut row = Vec::new();
        let mut current_fg = None;
        let mut current_bg = None;
        let mut pos = 0;
        let chars: Vec<char> = line.chars().collect();
        while pos < chars.len() {
            if chars[pos] == '\x1B' && pos + 1 < chars.len() && chars[pos+1] == '[' {
                if let Some(m_pos) = chars[pos..].iter().position(|&c| c == 'm') {
                    let seq = chars[pos..pos+m_pos+1].iter().collect::<String>();
                    if let Some(caps) = ansi_re.captures(&seq) {
                        let params = caps.get(1).map_or("", |m| m.as_str());
                        update_colors_local(params, &mut current_fg, &mut current_bg);
                    }
                    pos += m_pos + 1;
                    continue;
                }
            }
            let ch = chars[pos];
            if ch == ' ' && pos + 1 < chars.len() && chars[pos+1] == ' ' && current_bg.is_some() {
                row.push(Cell::Pixel { bg: current_bg.clone().unwrap() });
                pos += 2;
            } else if ch == ' ' && pos + 1 < chars.len() && chars[pos+1] == ' ' && current_bg.is_none() {
                row.push(Cell::Empty);
                pos += 2;
            } else {
                row.push(Cell::Styled { ch, fg: current_fg.clone(), bg: current_bg.clone() });
                pos += 1;
            }
        }
        grid.push(row);
    }
    grid
}

fn update_colors_local(params: &str, fg: &mut Option<Color>, bg: &mut Option<Color>) {
    if params.is_empty() || params == "0" {
        *fg = None; *bg = None; return;
    }
    let parts: Vec<u8> = params.split(';').filter_map(|s| s.parse().ok()).collect();
    let mut i = 0;
    while i < parts.len() {
        match parts[i] {
            0 => { *fg = None; *bg = None; }
            38 => {
                if i + 2 < parts.len() && parts[i+1] == 5 {
                    *fg = Some(Color::Indexed(parts[i+2])); i += 2;
                } else if i + 4 < parts.len() && parts[i+1] == 2 {
                    *fg = Some(Color::Rgb(parts[i+2], parts[i+3], parts[i+4])); i += 4;
                }
            }
            48 => {
                if i + 2 < parts.len() && parts[i+1] == 5 {
                    *bg = Some(Color::Indexed(parts[i+2])); i += 2;
                } else if i + 4 < parts.len() && parts[i+1] == 2 {
                    *bg = Some(Color::Rgb(parts[i+2], parts[i+3], parts[i+4])); i += 4;
                }
            }
            39 => *fg = None,
            49 => *bg = None,
            n if n >= 30 && n <= 37 => *fg = Some(Color::Named(n)),
            n if n >= 40 && n <= 47 => *bg = Some(Color::Named(n)),
            n if n >= 90 && n <= 97 => *fg = Some(Color::Named(n)),
            n if n >= 100 && n <= 107 => *bg = Some(Color::Named(n)),
            _ => {}
        }
        i += 1;
    }
}
