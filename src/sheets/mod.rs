pub mod cab3;
pub mod jarl;
pub mod summary;

use crate::{Error, Format, Item, Result};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Field {
    pub key: String,
    pub attrs: Vec<(String, String)>,
    pub value: String,
}

impl Field {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Field {
            key: key.into(),
            attrs: Vec::new(),
            value: value.into(),
        }
    }

    pub fn with(mut self, attr: impl Into<String>, value: impl Into<String>) -> Self {
        self.attrs.push((attr.into(), value.into()));
        self
    }

    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sheet {
    pub version: String,
    pub fields: Vec<Field>,
    pub table_type: Option<String>,
    pub table: Vec<u8>,
}

impl Sheet {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|f| f.key.eq_ignore_ascii_case(key))
            .map(|f| f.value.as_str())
    }

    pub fn all<'a>(&'a self, key: &'a str) -> impl Iterator<Item = &'a Field> + 'a {
        self.fields
            .iter()
            .filter(move |f| f.key.eq_ignore_ascii_case(key))
    }

    pub fn set(&mut self, key: &str, value: impl Into<String>) {
        let value = value.into();
        match self
            .fields
            .iter_mut()
            .find(|f| f.key.eq_ignore_ascii_case(key))
        {
            Some(f) => f.value = value,
            None => self.fields.push(Field::new(key, value)),
        }
    }

    pub fn push(&mut self, field: Field) {
        self.fields.push(field);
    }

    pub fn embed(&mut self, format: &dyn Format, items: &[Item]) -> Result<()> {
        self.table = format.write(items)?;
        Ok(())
    }

    pub fn items(&self, format: &dyn Format) -> Result<Vec<Item>> {
        format.read(&self.table)
    }
}

pub trait SheetFormat: Send + Sync {
    fn kind(&self) -> &'static str;

    fn name(&self) -> &'static str;

    fn extensions(&self) -> &'static [&'static str];

    fn decode(&self, bytes: &[u8]) -> Result<Sheet>;

    fn encode(&self, sheet: &Sheet) -> Result<Vec<u8>>;
}

pub fn sheets() -> Vec<Box<dyn SheetFormat>> {
    vec![
        Box::new(jarl::JarlSheet),
        Box::new(cab3::Cab3Sheet::default()),
    ]
}

pub fn sheet(name: &str) -> Option<Box<dyn SheetFormat>> {
    let all = sheets();
    let at = all
        .iter()
        .position(|f| f.kind() == name)
        .or_else(|| all.iter().position(|f| f.name() == name))
        .or_else(|| {
            all.iter()
                .position(|f| f.extensions().iter().any(|e| e.eq_ignore_ascii_case(name)))
        })?;
    all.into_iter().nth(at)
}

pub fn unpack(bytes: &[u8]) -> Result<Vec<Item>> {
    let mut causes = Vec::new();
    let tables = sheets()
        .iter()
        .filter_map(|s| match s.decode(bytes) {
            Ok(sheet) => Some(sheet.table),
            Err(e) => {
                causes.push(format!("[{}]: {e}", s.name()));
                None
            }
        })
        .chain(std::iter::once(bytes.to_vec()))
        .collect::<Vec<_>>();
    for table in tables {
        let mut empty = None;
        for f in crate::formats() {
            match f.read(&table) {
                Ok(items) if !items.is_empty() => return Ok(items),
                Ok(items) => empty = empty.or(Some(items)),
                Err(e) => causes.push(format!("[{}]: {e}", f.name())),
            }
        }
        if let Some(items) = empty {
            return Ok(items);
        }
    }
    Err(Error::new(causes.join("\n")))
}

pub(crate) fn sjis(text: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len());
    let mut buf = [0u8; 4];
    for c in text.chars() {
        let (bytes, _, bad) = encoding_rs::SHIFT_JIS.encode(c.encode_utf8(&mut buf));
        match bad {
            true => out.push(b'?'),
            false => out.extend_from_slice(&bytes),
        }
    }
    out
}
