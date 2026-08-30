use crate::language::LanguageProfile;

pub struct CommentPart {
    pub line: usize,
    pub col: usize,
    pub text: String,
}

pub struct Comment {
    pub parts: Vec<CommentPart>,
}

impl Comment {
    pub fn line(&self) -> usize {
        self.head().line
    }

    pub fn col(&self) -> usize {
        self.head().col
    }

    pub fn head(&self) -> &CommentPart {
        self.parts.first().expect("a comment has at least one part")
    }

    pub fn contains(&self, needle: &str) -> bool {
        self.parts.iter().any(|part| part.text.contains(needle))
    }
}

pub fn supports(language: &LanguageProfile) -> bool {
    language.comments.is_some()
}

pub fn snippet(value: &str) -> String {
    let trimmed = value.trim();
    let mut chars = trimmed.chars();
    let head: String = chars.by_ref().take(60).collect();
    if chars.next().is_some() {
        format!("{head}…")
    } else {
        head
    }
}

fn boundary_ok(marker: &str, previous: Option<char>) -> bool {
    match marker {
        "//" => previous != Some(':'),
        "#" | "--" => previous.is_none_or(char::is_whitespace),
        _ => true,
    }
}

fn rust_raw_string(rest: &str) -> Option<(usize, String)> {
    let prefix_len = if rest.starts_with("br") || rest.starts_with("cr") {
        2
    } else if rest.starts_with('r') {
        1
    } else {
        return None;
    };
    let mut cursor = prefix_len;
    while rest.as_bytes().get(cursor) == Some(&b'#') {
        cursor += 1;
    }
    if rest.as_bytes().get(cursor) != Some(&b'"') {
        return None;
    }
    let hashes = cursor - prefix_len;
    Some((cursor + 1, format!("\"{}", "#".repeat(hashes))))
}

struct OpenBlock {
    end: &'static str,
    parts: Vec<CommentPart>,
}

/// A comment that begins and ends on one line.
///
/// Three of the four places a comment is recorded build exactly this, from
/// three levels inside the traversal. Naming it keeps those sites flat enough
/// to read.
fn one_line(line: usize, col: usize, text: String) -> Comment {
    Comment {
        parts: vec![CommentPart { line, col, text }],
    }
}

/// The comments in a file.
pub fn scan(text: &str, language: &LanguageProfile) -> Vec<Comment> {
    walk(text, language).comments
}

/// The file with every comment and string literal blanked to spaces, one
/// entry per line.
///
/// Byte offsets and line lengths are preserved, so a column found in a masked
/// line is the column in the real one. This is what lets a rule match a
/// declaration without also matching it commented out or quoted inside a
/// string -- the two places source code most often appears without being
/// code. A language with no comment syntax comes back unmasked, having
/// nothing this can recognise.
pub fn code(text: &str, language: &LanguageProfile) -> Vec<String> {
    walk(text, language).code
}

/// Both views, from one traversal.
///
/// They are produced together because they answer one question -- where the
/// code stops and the prose starts -- and answering it in two places is how
/// the two answers come to disagree.
struct Walk {
    comments: Vec<Comment>,
    code: Vec<String>,
}

/// An unterminated single-quote string is taken to run to the end of its
/// line. The traversal does not carry one to the next line, and the mask does
/// not either, so the two always agree about where such a string stops.
fn walk(text: &str, language: &LanguageProfile) -> Walk {
    let Some(syntax) = language.comments else {
        return Walk {
            comments: Vec::new(),
            code: text.lines().map(str::to_owned).collect(),
        };
    };
    let mut comments = Vec::new();
    let mut code = Vec::new();
    let mut open_block: Option<OpenBlock> = None;
    let mut open_string: Option<String> = None;

    for (line_index, line) in text.lines().enumerate() {
        let line_number = line_index + 1;
        let mut holes: Vec<(usize, usize)> = Vec::new();

        'line_done: {
            if line_number == 1 && line.starts_with("#!") && syntax.line.contains(&"#") {
                break 'line_done;
            }

            let mut cursor = 0;
            if let Some(block) = open_block.as_mut() {
                match line.find(block.end) {
                    Some(position) => {
                        let end = position + block.end.len();
                        block.parts.push(CommentPart {
                            line: line_number,
                            col: 1,
                            text: line[..end].to_owned(),
                        });
                        let block = open_block.take().expect("open block exists");
                        comments.push(Comment { parts: block.parts });
                        holes.push((0, end));
                        cursor = end;
                    }
                    None => {
                        block.parts.push(CommentPart {
                            line: line_number,
                            col: 1,
                            text: line.to_owned(),
                        });
                        holes.push((0, line.len()));
                        break 'line_done;
                    }
                }
            } else if let Some(delimiter) = open_string.as_deref() {
                match line.find(delimiter) {
                    Some(position) => {
                        cursor = position + delimiter.len();
                        holes.push((0, cursor));
                        open_string = None;
                    }
                    None => {
                        holes.push((0, line.len()));
                        break 'line_done;
                    }
                }
            }

            let mut previous = line[..cursor].chars().next_back();
            let mut quote = None;
            let mut quote_start = 0;
            'line: while cursor < line.len() {
                let rest = &line[cursor..];
                let character = rest.chars().next().expect("cursor is a char boundary");

                if let Some(delimiter) = quote {
                    if character == '\\' {
                        cursor += character.len_utf8();
                        if let Some(escaped) = line[cursor..].chars().next() {
                            cursor += escaped.len_utf8();
                        }
                        continue;
                    }
                    cursor += character.len_utf8();
                    if character == delimiter {
                        quote = None;
                        holes.push((quote_start, cursor));
                    }
                    continue;
                }

                if language.id == "rust"
                    && let Some((opening_len, delimiter)) = rust_raw_string(rest)
                {
                    let after = cursor + opening_len;
                    match line[after..].find(&delimiter) {
                        Some(position) => {
                            let end = after + position + delimiter.len();
                            holes.push((cursor, end));
                            cursor = end;
                        }
                        None => {
                            open_string = Some(delimiter);
                            holes.push((cursor, line.len()));
                            break 'line;
                        }
                    }
                    previous = line[..cursor].chars().next_back();
                    continue;
                }

                if let Some(delimiter) = syntax
                    .multi_quotes
                    .iter()
                    .find(|delimiter| rest.starts_with(**delimiter))
                {
                    let after = cursor + delimiter.len();
                    match line[after..].find(delimiter) {
                        Some(position) => {
                            let end = after + position + delimiter.len();
                            holes.push((cursor, end));
                            cursor = end;
                        }
                        None => {
                            open_string = Some((*delimiter).to_owned());
                            holes.push((cursor, line.len()));
                            break 'line;
                        }
                    }
                    previous = delimiter.chars().next_back();
                    continue;
                }

                if syntax.quotes.contains(&character) {
                    quote = Some(character);
                    quote_start = cursor;
                    cursor += character.len_utf8();
                    continue;
                }

                if let Some((open, close)) =
                    syntax.block.iter().find(|(open, _)| rest.starts_with(open))
                {
                    let after = cursor + open.len();
                    match line[after..].find(close) {
                        Some(position) => {
                            let end = after + position + close.len();
                            comments.push(one_line(
                                line_number,
                                cursor + 1,
                                line[cursor..end].to_owned(),
                            ));
                            holes.push((cursor, end));
                            cursor = end;
                            previous = close.chars().next_back();
                            continue;
                        }
                        None => {
                            let opening = one_line(line_number, cursor + 1, rest.to_owned());
                            open_block = Some(OpenBlock {
                                end: close,
                                parts: opening.parts,
                            });
                            holes.push((cursor, line.len()));
                            break 'line;
                        }
                    }
                }

                if let Some(marker) = syntax.line.iter().find(|marker| rest.starts_with(**marker))
                    && boundary_ok(marker, previous)
                {
                    comments.push(one_line(line_number, cursor + 1, rest.to_owned()));
                    holes.push((cursor, line.len()));
                    break 'line;
                }

                previous = Some(character);
                cursor += character.len_utf8();
            }

            if quote.is_some() {
                holes.push((quote_start, line.len()));
            }
        }

        code.push(mask(line, &holes));
    }

    if let Some(block) = open_block {
        comments.push(Comment { parts: block.parts });
    }
    Walk { comments, code }
}

/// The line with the given byte ranges replaced by spaces.
///
/// Ranges always cover whole characters, so replacing their bytes with ASCII
/// spaces leaves valid UTF-8 and leaves every column where it was.
fn mask(line: &str, holes: &[(usize, usize)]) -> String {
    if holes.is_empty() {
        return line.to_owned();
    }
    let mut bytes = line.as_bytes().to_vec();
    for &(from, to) in holes {
        let from = from.min(bytes.len());
        let to = to.min(bytes.len());
        for byte in &mut bytes[from..to.max(from)] {
            *byte = b' ';
        }
    }
    String::from_utf8(bytes).expect("blanking whole characters leaves valid UTF-8")
}

/// The masked view keeps the shape of the line and drops only what is not
/// code. Every case here is one a rule reading `code` would otherwise get
/// wrong: a declaration commented out, quoted, or spanning a block.
#[cfg(test)]
mod tests {
    use super::code;
    use crate::language::language_profile;

    fn masked(source: &str, language: &str) -> Vec<String> {
        code(
            source,
            language_profile(language).expect("a language straitjacket knows"),
        )
    }

    #[test]
    fn code_survives_and_comments_do_not() {
        let source = "const MAX_SIZE: u8 = 3; // MAX_OTHER";
        let lines = masked(&format!("{source}\n"), "rust");

        assert!(lines[0].starts_with("const MAX_SIZE: u8 = 3;"));
        assert!(!lines[0].contains("MAX_OTHER"));
        assert_eq!(lines[0].len(), source.len(), "columns must not move");
    }

    #[test]
    fn a_string_literal_is_blanked_and_its_columns_are_kept() {
        let source = "let s = \"const MAX_SIZE = 3\";";
        let lines = masked(&format!("{source}\n"), "rust");

        assert!(lines[0].starts_with("let s = "));
        assert!(!lines[0].contains("MAX_SIZE"));
        assert!(lines[0].ends_with(';'), "the code after a string survives");
        assert_eq!(lines[0].len(), source.len(), "columns must not move");
    }

    #[test]
    fn a_block_comment_is_blanked_on_every_line_it_spans() {
        let lines = masked("a\n/* const MAX_SIZE = 1\n   still inside */ b\n", "rust");

        assert_eq!(lines[0], "a");
        assert_eq!(lines[1].trim(), "");
        assert_eq!(lines[2].trim(), "b");
    }

    #[test]
    fn a_rust_raw_string_is_blanked_hashes_and_all() {
        let source = "let s = r#\"const MAX_SIZE = 3\"#;";
        let lines = masked(&format!("{source}\n"), "rust");

        assert!(lines[0].starts_with("let s = "));
        assert!(!lines[0].contains("MAX_SIZE"));
        assert_eq!(lines[0].len(), source.len(), "columns must not move");
    }

    #[test]
    fn a_python_docstring_is_blanked_across_its_lines() {
        let lines = masked("x = 1\n\"\"\"\nMAX_SIZE = 3\n\"\"\"\ny = 2\n", "python");

        assert_eq!(lines[0], "x = 1");
        assert_eq!(lines[2].trim(), "");
        assert_eq!(lines[4], "y = 2");
    }

    #[test]
    fn a_language_with_no_comment_syntax_is_returned_whole() {
        let lines = masked("{\"MAX_SIZE\": 3}\n", "json");

        assert_eq!(lines, ["{\"MAX_SIZE\": 3}"]);
    }
}
