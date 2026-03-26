use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

const LUA_KEYWORDS: &[&str] = &[
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "goto", "if",
    "in", "local", "nil", "not", "or", "repeat", "return", "then", "true", "until", "while",
];

/// Tokenize a single line of Lua and return colored `Span`s.
pub fn highlight_lua_line(line: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // -- line comment
        if i + 1 < len && chars[i] == '-' && chars[i + 1] == '-' {
            let rest: String = chars[i..].iter().collect();
            spans.push(Span::styled(rest, Style::default().fg(Color::DarkGray)));
            break;
        }

        // String literals: "..." or '...'
        if chars[i] == '"' || chars[i] == '\'' {
            let quote = chars[i];
            let start = i;
            i += 1;
            while i < len && chars[i] != quote {
                if chars[i] == '\\' {
                    i += 1; // skip escaped char
                }
                i += 1;
            }
            if i < len {
                i += 1; // closing quote
            }
            let s: String = chars[start..i].iter().collect();
            spans.push(Span::styled(s, Style::default().fg(Color::Green)));
            continue;
        }

        // Numbers
        if chars[i].is_ascii_digit()
            || (chars[i] == '.' && i + 1 < len && chars[i + 1].is_ascii_digit())
        {
            let start = i;
            // hex prefix
            if chars[i] == '0' && i + 1 < len && (chars[i + 1] == 'x' || chars[i + 1] == 'X') {
                i += 2;
                while i < len && chars[i].is_ascii_hexdigit() {
                    i += 1;
                }
            } else {
                while i < len && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
            }
            let s: String = chars[start..i].iter().collect();
            spans.push(Span::styled(s, Style::default().fg(Color::Yellow)));
            continue;
        }

        // Identifiers and keywords
        if chars[i].is_ascii_alphabetic() || chars[i] == '_' {
            let start = i;
            while i < len && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            // Check for vim.xxx chain (vim.api, vim.fn, vim.g, vim.opt, vim.cmd, etc.)
            let word: String = chars[start..i].iter().collect();
            if word == "vim" && i < len && chars[i] == '.' {
                // Consume vim.xxx.yyy...
                while i < len && (chars[i] == '.' || chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let full: String = chars[start..i].iter().collect();
                spans.push(Span::styled(full, Style::default().fg(Color::Cyan)));
                continue;
            }
            if LUA_KEYWORDS.contains(&word.as_str()) {
                spans.push(Span::styled(
                    word,
                    Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                ));
            } else {
                spans.push(Span::raw(word));
            }
            continue;
        }

        // Whitespace runs
        if chars[i].is_whitespace() {
            let start = i;
            while i < len && chars[i].is_whitespace() {
                i += 1;
            }
            let s: String = chars[start..i].iter().collect();
            spans.push(Span::raw(s));
            continue;
        }

        // Single punctuation/operator character
        let s: String = chars[i..i + 1].iter().collect();
        spans.push(Span::raw(s));
        i += 1;
    }

    if spans.is_empty() {
        spans.push(Span::raw(String::new()));
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comment_highlighted() {
        let spans = highlight_lua_line("-- this is a comment");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].style.fg, Some(Color::DarkGray));
    }

    #[test]
    fn test_keyword_highlighted() {
        let spans = highlight_lua_line("local x = true");
        // "local" should be blue+bold
        let local_span = &spans[0];
        assert_eq!(local_span.style.fg, Some(Color::Blue));
        // "true" should also be blue+bold
        let true_span = spans.iter().find(|s| s.content == "true").unwrap();
        assert_eq!(true_span.style.fg, Some(Color::Blue));
    }

    #[test]
    fn test_string_highlighted() {
        let spans = highlight_lua_line(r#"local s = "hello""#);
        let str_span = spans.iter().find(|s| s.content.contains("hello")).unwrap();
        assert_eq!(str_span.style.fg, Some(Color::Green));
    }

    #[test]
    fn test_vim_api_highlighted() {
        let spans = highlight_lua_line("vim.api.nvim_create_autocmd");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].style.fg, Some(Color::Cyan));
    }

    #[test]
    fn test_number_highlighted() {
        let spans = highlight_lua_line("x = 42");
        let num_span = spans.iter().find(|s| s.content == "42").unwrap();
        assert_eq!(num_span.style.fg, Some(Color::Yellow));
    }

    #[test]
    fn test_mixed_line() {
        let spans = highlight_lua_line("local x = 10 -- count");
        assert!(spans.len() >= 4); // local, x, =, 10, -- count
        assert_eq!(spans[0].style.fg, Some(Color::Blue)); // local
        let comment = spans.last().unwrap();
        assert_eq!(comment.style.fg, Some(Color::DarkGray));
    }

    #[test]
    fn test_empty_line() {
        let spans = highlight_lua_line("");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "");
    }
}
