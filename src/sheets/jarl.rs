use crate::sheets::{sjis, Field, Sheet, SheetFormat};
use crate::text::{decode_sjis, NEWLINE};
use crate::{Error, Result};

pub const SUMMARY: &str = "SUMMARYSHEET";

pub const TABLE: &str = "LOGSHEET";

pub const VERSION: &str = "R2.1";

pub struct JarlSheet;

impl SheetFormat for JarlSheet {
    fn kind(&self) -> &'static str {
        "jarl"
    }

    fn name(&self) -> &'static str {
        "JARL R2.1"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["sum", "txt"]
    }

    fn decode(&self, bytes: &[u8]) -> Result<Sheet> {
        let text = decode_sjis(bytes);
        let mut sheet = Sheet::default();
        let mut rest = text.as_str();
        let mut found = false;
        while let Some((name, attrs, after)) = open_tag(rest) {
            if name.eq_ignore_ascii_case(SUMMARY) {
                found = true;
                sheet.version = attr(&attrs, "VERSION").unwrap_or_default();
                let (body, tail) = close_tag(after, SUMMARY)?;
                sheet.fields = fields(body)?;
                rest = tail;
            } else if name.eq_ignore_ascii_case(TABLE) {
                found = true;
                sheet.table_type = attr(&attrs, "TYPE");
                let after = after
                    .strip_prefix(NEWLINE)
                    .or_else(|| after.strip_prefix('\n'))
                    .unwrap_or(after);
                let (body, tail) = close_tag(after, TABLE)?;
                sheet.table = sjis(body);
                rest = tail;
            } else {
                let (_, tail) = close_tag(after, &name)?;
                rest = tail;
            }
        }
        match found {
            true => Ok(sheet),
            false => Err(Error::new(format!("<{SUMMARY}> is not found"))),
        }
    }

    fn encode(&self, sheet: &Sheet) -> Result<Vec<u8>> {
        let version = match sheet.version.as_str() {
            "" => VERSION,
            v => v,
        };
        let mut out = format!("<{SUMMARY} VERSION={version}>{NEWLINE}");
        for f in &sheet.fields {
            let attrs: String = f
                .attrs
                .iter()
                .map(|(k, v)| format!(" {k}={}", quote(v)))
                .collect();
            let value = crlf(&f.value);
            out.push_str(&format!("<{0}{attrs}>{value}</{0}>{NEWLINE}", f.key));
        }
        out.push_str(&format!("</{SUMMARY}>{NEWLINE}"));
        match &sheet.table_type {
            Some(t) => out.push_str(&format!("<{TABLE} TYPE=\"{t}\">{NEWLINE}")),
            None => out.push_str(&format!("<{TABLE}>{NEWLINE}")),
        }
        let mut bytes = sjis(&out);
        bytes.extend_from_slice(&sheet.table);
        if sheet.table.last().is_some_and(|b| *b != b'\n') {
            bytes.extend_from_slice(NEWLINE.as_bytes());
        }
        bytes.extend(format!("</{TABLE}>{NEWLINE}").bytes());
        Ok(bytes)
    }
}

fn crlf(text: &str) -> String {
    text.replace(NEWLINE, "\n").replace('\n', NEWLINE)
}

fn quote(value: &str) -> String {
    match value.is_empty() || value.contains([' ', '>']) {
        true => format!("\"{value}\""),
        false => value.to_string(),
    }
}

type Attrs = Vec<(String, String)>;

fn open_tag(text: &str) -> Option<(String, Attrs, &str)> {
    let mut rest = text;
    loop {
        let at = rest.find('<')?;
        let tail = &rest[at + 1..];
        let end = tail.find('>')?;
        let inner = &tail[..end];
        if inner.starts_with(['/', '!', '?']) {
            rest = &tail[end + 1..];
            continue;
        }
        let (name, attrs) = inner.split_once(char::is_whitespace).unwrap_or((inner, ""));
        return Some((name.to_string(), parse_attrs(attrs), &tail[end + 1..]));
    }
}

fn close_tag<'a>(text: &'a str, name: &str) -> Result<(&'a str, &'a str)> {
    let want = format!("</{}>", name.to_ascii_uppercase());
    let upper = text.to_ascii_uppercase();
    let at = upper
        .find(&want)
        .ok_or_else(|| Error::new(format!("<{name}> is not closed")))?;
    Ok((&text[..at], &text[at + want.len()..]))
}

fn parse_attrs(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = text.trim();
    while let Some((key, tail)) = rest.split_once('=') {
        let tail = tail.trim_start();
        let (value, next) = match tail.strip_prefix('"') {
            Some(q) => q.split_once('"').unwrap_or((q, "")),
            None => tail.split_once(char::is_whitespace).unwrap_or((tail, "")),
        };
        out.push((key.trim().to_string(), value.to_string()));
        rest = next.trim_start();
    }
    out
}

fn attr(attrs: &[(String, String)], name: &str) -> Option<String> {
    attrs
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.clone())
}

fn fields(body: &str) -> Result<Vec<Field>> {
    let mut out = Vec::new();
    let mut rest = body;
    while let Some((name, attrs, after)) = open_tag(rest) {
        let (value, tail) = close_tag(after, &name)?;
        out.push(Field {
            key: name,
            attrs,
            value: value.replace(NEWLINE, "\n").trim().to_string(),
        });
        rest = tail;
    }
    Ok(out)
}
