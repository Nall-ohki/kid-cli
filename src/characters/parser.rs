use std::collections::HashMap;
use regex::Regex;
use crate::characters::types::*;

#[allow(dead_code)]
pub fn parse(id: &str, content: &str, source: Source) -> anyhow::Result<Character> {
    let lines: Vec<&str> = content.lines().collect();
    
    // Human readable name
    let mut name = id.to_string();
    if name.starts_with("sxl-") { name = name[4..].to_string(); }
    name = name.replace('-', " ").replace('_', " ");
    name = name.chars().enumerate().map(|(i, c)| if i == 0 { c.to_uppercase().next().unwrap() } else { c }).collect();
    
    // 1. Strip comments and extract variables
    let mut vars = HashMap::new();
    let mut body_lines = Vec::new();
    let mut in_body = false;
    let mut body_marker = String::new();

    let var_re = Regex::new(r#"^\s*(\$\w+)\s*=\s*("[^"]*"|'[^']*'|[^\#;]+)"#)?;
    let body_start_re = Regex::new(r"^\s*(\$\w+)\s*=\s*<<(\w+);?")?;

    for line in &lines {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }

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

    if body_lines.is_empty() && !content.contains("<<") {
        return Err(anyhow::anyhow!("No character body found in {}", id));
    }

    // 2. Variable substitution and escape normalization
    let thoughts_replacement = match source {
        Source::CowFiles => "\u{E000}",
        Source::Charasay | Source::User => "\u{E000} ",
    };
    
    let mut resolved_lines = Vec::new();
    for line in body_lines {
        let mut resolved = line.to_string();
        
        // 1. Resolve variables
        let mut sorted_vars: Vec<_> = vars.iter().collect();
        sorted_vars.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        for (key, val) in sorted_vars {
            resolved = resolved.replace(key, val);
        }

        // 2. Resolve thoughts (aggressive)
        resolved = resolved.replace("$thoughts", thoughts_replacement);
        resolved = resolved.replace("$t", thoughts_replacement);

        // 3. Resolve escapes (aggressive)
        resolved = resolved.replace("\\e", "\x1B");
        resolved = resolved.replace("\\x1b", "\x1B");
        resolved = resolved.replace("\\x1B", "\x1B");
        resolved = normalize_unicode_escapes(&resolved);
        resolved = normalize_hex_escapes(&resolved);
        
        resolved_lines.push(resolved);
    }

    // Check for Sixel (either binary ESC P or literal \x1BP)
    let full_body = resolved_lines.join("\n");
    if full_body.contains("\x1BP") || full_body.contains("\\x1BP") {
        let data = full_body.replace("\\x1b", "\x1B").replace("\\x1B", "\x1B");
        
        let mut width = 0;
        let mut height = 0;
        // The cow files use \" before raster attrs, so handle optional backslash
        let sixel_re = Regex::new(r#"\x1BP[0-9;]*q\\?"1;1;(\d+);(\d+)"#)?;
        if let Some(caps) = sixel_re.captures(&data) {
            width = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
            height = caps.get(2).unwrap().as_str().parse().unwrap_or(0);
        }

        // Extract only the Sixel DCS sequence (ESC P ... ESC \)
        // Strip connector lines, leading spaces, and cursor-movement prefixes
        let mut sixel_data = String::new();
        for line in data.lines() {
            if let Some(pos) = line.find("\x1BP") {
                sixel_data.push_str(&line[pos..]);
            }
        }
        // Resolve Perl-style escapes: \" -> " and \\\\ -> \\
        let sixel_data = sixel_data.replace("\\\"", "\"").replace("\\\\", "\\");

         return Ok(Character {
            id: id.to_string(),
            name,
            source,
            kind: CharacterKind::Sixel(sixel_data),
            height: if height > 0 { (height / 10) as usize } else { resolved_lines.len() },
            width: if width > 0 { (width / 5) as usize } else { 0 },
        });
    }

    // 3. Construct Cell Grid
    let grid = construct_grid(&resolved_lines);
    let height = grid.len();
    let width = grid.iter().map(|r| r.len()).max().unwrap_or(0);

    Ok(Character {
        id: id.to_string(),
        name,
        source,
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
                        update_colors(params, &mut current_fg, &mut current_bg);
                    }
                    pos += m_pos + 1;
                    continue;
                }
            }

            let ch = chars[pos];
            
            if ch == '\u{E000}' {
                row.push(Cell::Connector { ch: '\\' });
            } else {
                row.push(Cell::Styled {
                    ch,
                    fg: current_fg.clone(),
                    bg: current_bg.clone(),
                });
            }
            pos += 1;
        }
        grid.push(row);
    }
    grid
}

fn update_colors(params: &str, fg: &mut Option<Color>, bg: &mut Option<Color>) {
    if params.is_empty() || params == "0" {
        *fg = None;
        *bg = None;
        return;
    }

    let parts: Vec<u8> = params.split(';').filter_map(|s| s.parse().ok()).collect();
    let mut i = 0;
    while i < parts.len() {
        match parts[i] {
            0 => { *fg = None; *bg = None; }
            38 => {
                if i + 2 < parts.len() && parts[i+1] == 5 {
                    *fg = Some(Color::Indexed(parts[i+2]));
                    i += 2;
                } else if i + 4 < parts.len() && parts[i+1] == 2 {
                    *fg = Some(Color::Rgb(parts[i+2], parts[i+3], parts[i+4]));
                    i += 4;
                }
            }
            48 => {
                if i + 2 < parts.len() && parts[i+1] == 5 {
                    *bg = Some(Color::Indexed(parts[i+2]));
                    i += 2;
                } else if i + 4 < parts.len() && parts[i+1] == 2 {
                    *bg = Some(Color::Rgb(parts[i+2], parts[i+3], parts[i+4]));
                    i += 4;
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
    fn test_strip_comments_preserves_inline() {
        let content = "$a = \"\\e[48;5;233m  \"; # comment\n$the_chara = <<EOC\n$a\nEOC";
        let chara = parse("test", content, Source::CowFiles).unwrap();
        if let CharacterKind::Grid(grid) = chara.kind {
            assert_eq!(grid[0][0], Cell::Styled { ch: ' ', fg: None, bg: Some(Color::Indexed(233)) });
        }
    }

    #[test]
    fn test_extract_variables_chara() {
        let content = "$a = \"\\e[48;5;233m  \";\n$the_chara = <<EOC\n$a\nEOC";
        let chara = parse("test", content, Source::CowFiles).unwrap();
        if let CharacterKind::Grid(grid) = chara.kind {
            assert_eq!(grid[0][0], Cell::Styled { ch: ' ', fg: None, bg: Some(Color::Indexed(233)) });
        }
    }

    #[test]
    fn test_extract_variables_case_sensitive() {
        let content = "$a = \"\\e[48;5;1m  \";\n$A = \"\\e[48;5;2m  \";\n$the_chara = <<EOC\n$a$A\nEOC";
        let chara = parse("test", content, Source::CowFiles).unwrap();
        if let CharacterKind::Grid(grid) = chara.kind {
            assert_eq!(grid[0][0], Cell::Styled { ch: ' ', fg: None, bg: Some(Color::Indexed(1)) });
            assert_eq!(grid[0][2], Cell::Styled { ch: ' ', fg: None, bg: Some(Color::Indexed(2)) });
        }
    }

    #[test]
    fn test_extract_body_chara() {
        let content = "$the_chara = <<EOC\nbody\nEOC";
        let chara = parse("test", content, Source::CowFiles).unwrap();
        assert_eq!(chara.height, 1);
    }

    #[test]
    fn test_extract_body_cow() {
        let content = "$the_cow = <<EOC;\nbody\nEOC";
        let chara = parse("test", content, Source::CowFiles).unwrap();
        assert_eq!(chara.height, 1);
    }

    #[test]
    fn test_variable_substitution() {
        let content = "$a = \"pixel\";\n$the_cow = <<EOC\n$a\nEOC";
        let chara = parse("test", content, Source::CowFiles).unwrap();
        if let CharacterKind::Grid(grid) = chara.kind {
             assert_eq!(grid[0].len(), 5);
        }
    }

    #[test]
    fn test_escape_normalization_backslash_e() {
        let content = "$the_cow = <<EOC\n\\e[48;5;208m  \nEOC";
        let chara = parse("test", content, Source::CowFiles).unwrap();
        if let CharacterKind::Grid(grid) = chara.kind {
            assert_eq!(grid[0][0], Cell::Styled { ch: ' ', fg: None, bg: Some(Color::Indexed(208)) });
        }
    }

    #[test]
    fn test_escape_normalization_unicode() {
        let s = "\\x{1FB35}";
        let normalized = normalize_unicode_escapes(s);
        assert_eq!(normalized, "🬵");
        
        let s2 = "\\x{1CD96}";
        let normalized2 = normalize_unicode_escapes(s2);
        assert_eq!(normalized2, "\u{1CD96}");
    }

    #[test]
    fn test_sixel_detection() {
        let content = "$the_cow = <<EOC\n\\x1BP0;1q...\nEOC";
        let chara = parse("test", content, Source::CowFiles).unwrap();
        assert!(matches!(chara.kind, CharacterKind::Sixel(_)));
    }

    #[test]
    fn test_parse_pixel_sequence() {
        let lines = vec!["\x1B[48;5;208m  ".to_string()];
        let grid = construct_grid(&lines);
        assert_eq!(grid[0].len(), 2);
        assert_eq!(grid[0][0], Cell::Styled { ch: ' ', fg: None, bg: Some(Color::Indexed(208)) });
    }

    #[test]
    fn test_parse_reset_as_empty() {
        let lines = vec!["\x1B[49m  ".to_string()];
        let grid = construct_grid(&lines);
        assert_eq!(grid[0][0], Cell::Styled { ch: ' ', fg: None, bg: None });
    }

    #[test]
    fn test_parse_styled_char() {
        let lines = vec!["\x1B[38;5;16m█".to_string()];
        let grid = construct_grid(&lines);
        assert_eq!(grid[0][0], Cell::Styled { ch: '█', fg: Some(Color::Indexed(16)), bg: None });
    }

    #[test]
    fn test_parse_fg_and_bg() {
        let lines = vec!["\x1B[38;5;16;48;5;231m█".to_string()];
        let grid = construct_grid(&lines);
        assert_eq!(grid[0][0], Cell::Styled { ch: '█', fg: Some(Color::Indexed(16)), bg: Some(Color::Indexed(231)) });
    }

    #[test]
    fn test_parse_named_colors() {
        let lines = vec!["\x1B[37m_".to_string()];
        let grid = construct_grid(&lines);
        assert_eq!(grid[0][0], Cell::Styled { ch: '_', fg: Some(Color::Named(37)), bg: None });
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

    #[test]
    fn test_grid_dimensions() {
        let content = "$the_cow = <<EOC\nline1\nline123\nEOC";
        let chara = parse("test", content, Source::CowFiles).unwrap();
        assert_eq!(chara.height, 2);
        assert_eq!(chara.width, 7);
    }

    #[test]
    fn test_extract_variables_cow() {
        let content = "## comment\n$the_cow = <<EOC\nbody\nEOC";
        let chara = parse("test", content, Source::CowFiles).unwrap();
        assert_eq!(chara.height, 1);
    }

    #[test]
    fn test_extract_body_empty() {
        let content = "$a = 1;";
        let res = parse("test", content, Source::CowFiles);
        assert!(res.is_err());
    }

    #[test]
    fn test_variable_substitution_ordering() {
        let content = "$a = \"short\";\n$aa = \"long\";\n$the_cow = <<EOC\n$aa $a\nEOC";
        let chara = parse("test", content, Source::CowFiles).unwrap();
        // $aa should be replaced by "long", not "$a" being replaced by "short" inside "$aa"
        if let CharacterKind::Grid(grid) = chara.kind {
             assert_eq!(grid[0].len(), 10); // "long" (4) + " " (1) + "short" (5)
        }
    }

    #[test]
    fn test_escape_normalization_hex_passthrough() {
        let content = "$the_cow = <<EOC\n\\x1B[37m_\nEOC";
        let chara = parse("test", content, Source::CowFiles).unwrap();
        if let CharacterKind::Grid(grid) = chara.kind {
            assert_eq!(grid[0][0], Cell::Styled { ch: '_', fg: Some(Color::Named(37)), bg: None });
        }
    }

    #[test]
    fn test_parse_plain_text() {
        let lines = vec!["hello".to_string()];
        let grid = construct_grid(&lines);
        assert_eq!(grid[0].len(), 5);
        assert_eq!(grid[0][0], Cell::Styled { ch: 'h', fg: None, bg: None });
    }

    #[test]
    fn test_parse_mixed_line() {
        // From clippit.cow: \x1B[90m(\x1B[97mo\x1B[90m)
        let lines = vec!["\x1B[90m(\x1B[97mo\x1B[90m)".to_string()];
        let grid = construct_grid(&lines);
        assert_eq!(grid[0].len(), 3);
        assert_eq!(grid[0][0], Cell::Styled { ch: '(', fg: Some(Color::Named(90)), bg: None });
        assert_eq!(grid[0][1], Cell::Styled { ch: 'o', fg: Some(Color::Named(97)), bg: None });
    }

    #[test]
    fn test_parse_full_ferris() {
        let path = "assets/characters/ferris.chara";
        if let Ok(content) = std::fs::read_to_string(path) {
            let chara = parse("ferris", &content, Source::CowFiles).unwrap();
            assert_eq!(chara.id, "ferris");
            assert!(chara.height > 10);
        }
    }

    #[test]
    fn test_parse_full_clippit() {
        let path = "assets/characters/clippit.cow";
        if let Ok(content) = std::fs::read_to_string(path) {
            let chara = parse("clippit", &content, Source::CowFiles).unwrap();
            assert_eq!(chara.id, "clippit");
        }
    }

    #[test]
    fn test_parse_full_alexkidd() {
        let path = "assets/characters/alexkidd.cow";
        if let Ok(content) = std::fs::read_to_string(path) {
            let chara = parse("alexkidd", &content, Source::CowFiles).unwrap();
            assert_eq!(chara.id, "alexkidd");
        }
    }
}
