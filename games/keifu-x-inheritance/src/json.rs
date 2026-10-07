//! A small JSON reader for the content files, and typed accessors that fail loudly.
//!
//! The spec's content is JSON (`spec/content/README.md`), and the engine reads
//! no data format of its own, so the game carries this small reader rather
//! than a new dependency (FINDINGS G-036). It parses the whole of RFC 8259 that
//! the content uses — objects, arrays, strings with escapes, numbers, booleans,
//! null — and keeps object keys in file order, because the content's order is
//! meaningful (creation order, formation order, index order).
//!
//! Every accessor names the path it was reading, so a schema mismatch reports
//! `household.json: heroes[3].fear.dread: expected an integer, found a string`
//! rather than a bare "type error".

/// One parsed JSON value. Objects keep their keys in file order.
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    /// `null`.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// Every number, as the content's numbers are small integers and short decimals.
    Number(f64),
    /// A string, escapes resolved.
    Str(String),
    /// An array.
    Array(Vec<Json>),
    /// An object, in file order.
    Object(Vec<(String, Json)>),
}

/// A content file that does not say what the schema says it must.
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaError {
    /// Where, as `file: path`.
    pub at: String,
    /// What was wrong.
    pub what: String,
}

impl std::fmt::Display for SchemaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[keifu] content does not match its schema\n  {}: {}\n  likely cause: a file \
             under games/keifu-x-inheritance/spec/content/ changed shape, or the loader names a key the \
             schema does not have\n  fix: compare the file with spec/content/README.md and \
             correct whichever side is wrong (the spec is read-only to a port session)",
            self.at, self.what
        )
    }
}

/// A located value: the value plus the path that reached it, for error messages.
#[derive(Clone)]
pub struct At<'a> {
    /// The value.
    pub value: &'a Json,
    path: String,
}

/// Parse `text`; `file` names it in any error.
pub fn parse(file: &str, text: &str) -> Result<Json, SchemaError> {
    let mut parser = Parser {
        bytes: text.as_bytes(),
        at: 0,
    };
    let value = parser.value().map_err(|what| SchemaError {
        at: format!("{file}: byte {}", parser.at),
        what,
    })?;
    parser.space();
    if parser.at != parser.bytes.len() {
        return Err(SchemaError {
            at: format!("{file}: byte {}", parser.at),
            what: "trailing characters after the document".to_owned(),
        });
    }
    Ok(value)
}

impl<'a> At<'a> {
    /// The root of a file.
    pub fn root(value: &'a Json, file: &str) -> Self {
        Self {
            value,
            path: file.to_owned(),
        }
    }

    fn error(&self, what: String) -> SchemaError {
        SchemaError {
            at: self.path.clone(),
            what,
        }
    }

    fn kind(&self) -> &'static str {
        match self.value {
            Json::Null => "null",
            Json::Bool(_) => "a boolean",
            Json::Number(_) => "a number",
            Json::Str(_) => "a string",
            Json::Array(_) => "an array",
            Json::Object(_) => "an object",
        }
    }

    /// A required key of an object.
    pub fn key(&self, key: &str) -> Result<At<'a>, SchemaError> {
        self.find(key)?
            .ok_or_else(|| self.error(format!("missing required key {key:?}")))
    }

    /// An optional key of an object (absent and `null` both read as `None`).
    pub fn find(&self, key: &str) -> Result<Option<At<'a>>, SchemaError> {
        let Json::Object(entries) = self.value else {
            return Err(self.error(format!(
                "expected an object holding {key:?}, found {}",
                self.kind()
            )));
        };
        Ok(entries
            .iter()
            .find(|(name, value)| name == key && !matches!(value, Json::Null))
            .map(|(_, value)| At {
                value,
                path: format!("{}.{key}", self.path),
            }))
    }

    /// The entries of an object, in file order.
    pub fn entries(&self) -> Result<Vec<(String, At<'a>)>, SchemaError> {
        let Json::Object(entries) = self.value else {
            return Err(self.error(format!("expected an object, found {}", self.kind())));
        };
        Ok(entries
            .iter()
            .map(|(name, value)| {
                (
                    name.clone(),
                    At {
                        value,
                        path: format!("{}.{name}", self.path),
                    },
                )
            })
            .collect())
    }

    /// The elements of an array.
    pub fn items(&self) -> Result<Vec<At<'a>>, SchemaError> {
        let Json::Array(items) = self.value else {
            return Err(self.error(format!("expected an array, found {}", self.kind())));
        };
        Ok(items
            .iter()
            .enumerate()
            .map(|(index, value)| At {
                value,
                path: format!("{}[{index}]", self.path),
            })
            .collect())
    }

    /// A string.
    pub fn str(&self) -> Result<String, SchemaError> {
        match self.value {
            Json::Str(text) => Ok(text.clone()),
            _ => Err(self.error(format!("expected a string, found {}", self.kind()))),
        }
    }

    /// An integer (a number with no fractional part).
    pub fn int(&self) -> Result<i32, SchemaError> {
        match self.value {
            Json::Number(n) if n.fract() == 0.0 && n.abs() < 1e9 => Ok(*n as i32),
            _ => Err(self.error(format!("expected an integer, found {}", self.kind()))),
        }
    }

    /// A boolean.
    pub fn bool(&self) -> Result<bool, SchemaError> {
        match self.value {
            Json::Bool(flag) => Ok(*flag),
            _ => Err(self.error(format!("expected a boolean, found {}", self.kind()))),
        }
    }

    /// A schema error at this value, for a check the accessors cannot express.
    pub fn reject(&self, what: String) -> SchemaError {
        self.error(what)
    }
}

/// A byte cursor over the document.
struct Parser<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Parser<'_> {
    fn space(&mut self) {
        while self
            .bytes
            .get(self.at)
            .is_some_and(|b| matches!(b, b' ' | b'\n' | b'\r' | b'\t'))
        {
            self.at += 1;
        }
    }

    fn eat(&mut self, byte: u8) -> Result<(), String> {
        self.space();
        if self.bytes.get(self.at) == Some(&byte) {
            self.at += 1;
            Ok(())
        } else {
            Err(format!("expected {:?}", byte as char))
        }
    }

    fn literal(&mut self, word: &str, value: Json) -> Result<Json, String> {
        if self.bytes[self.at..].starts_with(word.as_bytes()) {
            self.at += word.len();
            Ok(value)
        } else {
            Err(format!("expected {word}"))
        }
    }

    fn value(&mut self) -> Result<Json, String> {
        self.space();
        match self.bytes.get(self.at) {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => self.string().map(Json::Str),
            Some(b't') => self.literal("true", Json::Bool(true)),
            Some(b'f') => self.literal("false", Json::Bool(false)),
            Some(b'n') => self.literal("null", Json::Null),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(other) => Err(format!("unexpected character {:?}", *other as char)),
            None => Err("the document ended early".to_owned()),
        }
    }

    fn object(&mut self) -> Result<Json, String> {
        self.eat(b'{')?;
        let mut entries = Vec::new();
        self.space();
        if self.bytes.get(self.at) == Some(&b'}') {
            self.at += 1;
            return Ok(Json::Object(entries));
        }
        loop {
            self.space();
            let key = self.string()?;
            if entries.iter().any(|(name, _)| *name == key) {
                return Err(format!("duplicate key {key:?}"));
            }
            self.eat(b':')?;
            let value = self.value()?;
            entries.push((key, value));
            self.space();
            match self.bytes.get(self.at) {
                Some(b',') => self.at += 1,
                Some(b'}') => {
                    self.at += 1;
                    return Ok(Json::Object(entries));
                }
                _ => return Err("expected ',' or '}' in an object".to_owned()),
            }
        }
    }

    fn array(&mut self) -> Result<Json, String> {
        self.eat(b'[')?;
        let mut items = Vec::new();
        self.space();
        if self.bytes.get(self.at) == Some(&b']') {
            self.at += 1;
            return Ok(Json::Array(items));
        }
        loop {
            items.push(self.value()?);
            self.space();
            match self.bytes.get(self.at) {
                Some(b',') => self.at += 1,
                Some(b']') => {
                    self.at += 1;
                    return Ok(Json::Array(items));
                }
                _ => return Err("expected ',' or ']' in an array".to_owned()),
            }
        }
    }

    fn string(&mut self) -> Result<String, String> {
        if self.bytes.get(self.at) != Some(&b'"') {
            return Err("expected a string".to_owned());
        }
        self.at += 1;
        let mut out: Vec<u8> = Vec::new();
        loop {
            let Some(&byte) = self.bytes.get(self.at) else {
                return Err("a string was not closed".to_owned());
            };
            self.at += 1;
            match byte {
                b'"' => break,
                b'\\' => {
                    let Some(&escape) = self.bytes.get(self.at) else {
                        return Err("a string ended inside an escape".to_owned());
                    };
                    self.at += 1;
                    match escape {
                        b'"' | b'\\' | b'/' => out.push(escape),
                        b'n' => out.push(b'\n'),
                        b't' => out.push(b'\t'),
                        b'r' => out.push(b'\r'),
                        b'b' => out.push(8),
                        b'f' => out.push(12),
                        b'u' => {
                            let hex = self
                                .bytes
                                .get(self.at..self.at + 4)
                                .and_then(|h| std::str::from_utf8(h).ok())
                                .and_then(|h| u32::from_str_radix(h, 16).ok())
                                .ok_or("a \\u escape is not four hex digits")?;
                            self.at += 4;
                            let ch = char::from_u32(hex).ok_or("a \\u escape is a surrogate")?;
                            let mut buf = [0; 4];
                            out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                        }
                        other => return Err(format!("unknown escape \\{}", other as char)),
                    }
                }
                _ => out.push(byte),
            }
        }
        String::from_utf8(out).map_err(|_| "a string is not UTF-8".to_owned())
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.at;
        while self
            .bytes
            .get(self.at)
            .is_some_and(|b| matches!(b, b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9'))
        {
            self.at += 1;
        }
        std::str::from_utf8(&self.bytes[start..self.at])
            .ok()
            .and_then(|text| text.parse::<f64>().ok())
            .map(Json::Number)
            .ok_or_else(|| "a malformed number".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_document_keeps_its_object_keys_in_file_order() {
        let Ok(doc) = parse("t", r#"{"b": 1, "a": [true, null, "x\"y"], "c": -2.5}"#) else {
            panic!("did not parse");
        };
        let Json::Object(entries) = doc else {
            panic!("not an object")
        };
        let keys: Vec<&str> = entries.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(keys, ["b", "a", "c"]);
        assert_eq!(
            entries[1].1,
            Json::Array(vec![
                Json::Bool(true),
                Json::Null,
                Json::Str("x\"y".to_owned())
            ])
        );
    }

    #[test]
    fn a_missing_key_names_the_path_that_reached_it() {
        let Ok(doc) = parse("f.json", r#"{"heroes": [{"name": 3}]}"#) else {
            panic!("did not parse");
        };
        let root = At::root(&doc, "f.json");
        let Ok(heroes) = root.key("heroes") else {
            panic!("no heroes")
        };
        let Ok(items) = heroes.items() else {
            panic!("no items")
        };
        let Err(error) = items[0].key("age") else {
            panic!("found an age that is not there");
        };
        assert_eq!(error.at, "f.json.heroes[0]");
        let Err(error) = items[0].key("name").and_then(|n| n.str()) else {
            panic!("a number read as a string");
        };
        assert_eq!(error.at, "f.json.heroes[0].name");
    }

    #[test]
    fn trailing_garbage_and_duplicate_keys_are_rejected() {
        assert!(parse("t", "{} x").is_err());
        assert!(parse("t", r#"{"a":1,"a":2}"#).is_err());
        assert!(parse("t", r#"{"a":"\\%"}"#).is_ok());
    }
}
