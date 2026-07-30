use entl_codebase::LanguageProfile;

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

pub fn scan(text: &str, language: &LanguageProfile) -> Vec<Comment> {
    let Some(syntax) = language.comments else {
        return Vec::new();
    };
    let mut comments = Vec::new();
    let mut open_block: Option<OpenBlock> = None;
    let mut open_string: Option<String> = None;

    for (line_index, line) in text.lines().enumerate() {
        let line_number = line_index + 1;
        if line_number == 1 && line.starts_with("#!") && syntax.line.contains(&"#") {
            continue;
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
                    cursor = end;
                }
                None => {
                    block.parts.push(CommentPart {
                        line: line_number,
                        col: 1,
                        text: line.to_owned(),
                    });
                    continue;
                }
            }
        } else if let Some(delimiter) = open_string.as_deref() {
            match line.find(delimiter) {
                Some(position) => {
                    cursor = position + delimiter.len();
                    open_string = None;
                }
                None => continue,
            }
        }

        let mut previous = line[..cursor].chars().next_back();
        let mut quote = None;
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
                if character == delimiter {
                    quote = None;
                }
                cursor += character.len_utf8();
                continue;
            }

            if language.id == "rust"
                && let Some((opening_len, delimiter)) = rust_raw_string(rest)
            {
                let after = cursor + opening_len;
                match line[after..].find(&delimiter) {
                    Some(position) => cursor = after + position + delimiter.len(),
                    None => {
                        open_string = Some(delimiter);
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
                    Some(position) => cursor = after + position + delimiter.len(),
                    None => {
                        open_string = Some((*delimiter).to_owned());
                        break 'line;
                    }
                }
                previous = delimiter.chars().next_back();
                continue;
            }

            if syntax.quotes.contains(&character) {
                quote = Some(character);
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
                        comments.push(Comment {
                            parts: vec![CommentPart {
                                line: line_number,
                                col: cursor + 1,
                                text: line[cursor..end].to_owned(),
                            }],
                        });
                        cursor = end;
                        previous = close.chars().next_back();
                        continue;
                    }
                    None => {
                        open_block = Some(OpenBlock {
                            end: close,
                            parts: vec![CommentPart {
                                line: line_number,
                                col: cursor + 1,
                                text: rest.to_owned(),
                            }],
                        });
                        break 'line;
                    }
                }
            }

            if let Some(marker) = syntax.line.iter().find(|marker| rest.starts_with(**marker))
                && boundary_ok(marker, previous)
            {
                comments.push(Comment {
                    parts: vec![CommentPart {
                        line: line_number,
                        col: cursor + 1,
                        text: rest.to_owned(),
                    }],
                });
                break 'line;
            }

            previous = Some(character);
            cursor += character.len_utf8();
        }
    }

    if let Some(block) = open_block {
        comments.push(Comment { parts: block.parts });
    }
    comments
}
