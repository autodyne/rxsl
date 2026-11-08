use std::cmp::Ordering;

use crate::{Error, Result};

const MAGIC: &[u8; 16] = b"SQLite format 3\0";
const PAGE: usize = 4096;
const HEAD: usize = 100;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Int(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

impl Value {
    pub fn text(&self) -> Option<String> {
        match self {
            Value::Null => None,
            Value::Int(n) => Some(n.to_string()),
            Value::Real(r) => Some(r.to_string()),
            Value::Text(t) => Some(t.clone()),
            Value::Blob(b) => Some(String::from_utf8_lossy(b).into_owned()),
        }
    }

    pub fn int(&self) -> Option<i64> {
        match self {
            Value::Int(n) => Some(*n),
            Value::Real(r) => Some(*r as i64),
            Value::Text(t) => t.trim().parse().ok(),
            _ => None,
        }
    }

    pub fn real(&self) -> Option<f64> {
        match self {
            Value::Int(n) => Some(*n as f64),
            Value::Real(r) => Some(*r),
            Value::Text(t) => t.trim().parse().ok(),
            _ => None,
        }
    }

    fn cmp(&self, other: &Value) -> Ordering {
        fn rank(v: &Value) -> u8 {
            match v {
                Value::Null => 0,
                Value::Int(_) | Value::Real(_) => 1,
                Value::Text(_) => 2,
                Value::Blob(_) => 3,
            }
        }
        match (self, other) {
            (Value::Text(a), Value::Text(b)) => a.as_bytes().cmp(b.as_bytes()),
            (Value::Blob(a), Value::Blob(b)) => a.cmp(b),
            (a, b) if rank(a) == 1 && rank(b) == 1 => {
                let (x, y) = (a.real().unwrap_or(0.0), b.real().unwrap_or(0.0));
                x.partial_cmp(&y).unwrap_or(Ordering::Equal)
            }
            (a, b) => rank(a).cmp(&rank(b)),
        }
    }
}

fn bad(text: &str) -> Error {
    Error::new(format!("malformed SQLite database: {text}"))
}

pub fn get_varint(bytes: &[u8], at: usize) -> Result<(i64, usize)> {
    let mut value: u64 = 0;
    for i in 0..9 {
        let b = *bytes.get(at + i).ok_or_else(|| bad("varint"))?;
        if i == 8 {
            return Ok(((value << 8 | b as u64) as i64, 9));
        }
        value = value << 7 | (b & 0x7F) as u64;
        if b & 0x80 == 0 {
            return Ok((value as i64, i + 1));
        }
    }
    Err(bad("varint"))
}

pub fn put_varint(out: &mut Vec<u8>, value: i64) {
    let v = value as u64;
    if v >> 56 != 0 {
        let mut head = [0u8; 8];
        let mut rest = v >> 8;
        for i in (0..8).rev() {
            head[i] = (rest & 0x7F) as u8 | 0x80;
            rest >>= 7;
        }
        out.extend(head);
        out.push(v as u8);
        return;
    }
    let mut groups = vec![(v & 0x7F) as u8];
    let mut rest = v >> 7;
    while rest != 0 {
        groups.push((rest & 0x7F) as u8 | 0x80);
        rest >>= 7;
    }
    out.extend(groups.iter().rev());
}

fn be(bytes: &[u8], at: usize, n: usize) -> Result<u64> {
    let s = bytes.get(at..at + n).ok_or_else(|| bad("truncated"))?;
    Ok(s.iter().fold(0, |a, b| a << 8 | *b as u64))
}

pub fn decode_record(payload: &[u8]) -> Result<Vec<Value>> {
    let (size, mut at) = get_varint(payload, 0)?;
    let mut types = Vec::new();
    while at < size as usize {
        let (t, n) = get_varint(payload, at)?;
        types.push(t);
        at += n;
    }
    let mut pos = size as usize;
    let mut out = Vec::with_capacity(types.len());
    for t in types {
        let int = |n: usize| -> Result<i64> {
            let raw = be(payload, pos, n)?;
            let shift = 64 - 8 * n as u32;
            Ok(((raw << shift) as i64) >> shift)
        };
        let (value, len) = match t {
            0 => (Value::Null, 0),
            1..=4 => (Value::Int(int(t as usize)?), t as usize),
            5 => (Value::Int(int(6)?), 6),
            6 => (Value::Int(int(8)?), 8),
            7 => (Value::Real(f64::from_bits(be(payload, pos, 8)?)), 8),
            8 => (Value::Int(0), 0),
            9 => (Value::Int(1), 0),
            t if t >= 12 => {
                let n = ((t - 12) / 2) as usize;
                let s = payload.get(pos..pos + n).ok_or_else(|| bad("record"))?;
                match t % 2 {
                    0 => (Value::Blob(s.to_vec()), n),
                    _ => (Value::Text(String::from_utf8_lossy(s).into_owned()), n),
                }
            }
            _ => return Err(bad("serial type")),
        };
        out.push(value);
        pos += len;
    }
    Ok(out)
}

pub fn encode_record(values: &[Value]) -> Vec<u8> {
    let mut head = Vec::new();
    let mut body = Vec::new();
    for v in values {
        let t = match v {
            Value::Null => 0,
            Value::Int(0) => 8,
            Value::Int(1) => 9,
            Value::Int(n) => {
                let (t, w) = match *n {
                    -128..=127 => (1, 1),
                    -32768..=32767 => (2, 2),
                    -8388608..=8388607 => (3, 3),
                    -2147483648..=2147483647 => (4, 4),
                    -140737488355328..=140737488355327 => (5, 6),
                    _ => (6, 8),
                };
                body.extend(&n.to_be_bytes()[8 - w..]);
                t
            }
            Value::Real(r) => {
                body.extend(r.to_bits().to_be_bytes());
                7
            }
            Value::Text(s) => {
                body.extend(s.as_bytes());
                s.len() as i64 * 2 + 13
            }
            Value::Blob(b) => {
                body.extend(b);
                b.len() as i64 * 2 + 12
            }
        };
        put_varint(&mut head, t);
    }
    let mut size = head.len() + 1;
    loop {
        let mut v = Vec::new();
        put_varint(&mut v, size as i64);
        if v.len() + head.len() == size {
            break;
        }
        size = v.len() + head.len();
    }
    let mut out = Vec::new();
    put_varint(&mut out, size as i64);
    out.extend(head);
    out.extend(body);
    out
}

pub struct Table {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
}

impl Table {
    pub fn column(&self, name: &str) -> Option<usize> {
        self.columns
            .iter()
            .position(|c| c.eq_ignore_ascii_case(name))
    }
}

pub struct Reader<'a> {
    bytes: &'a [u8],
    page: usize,
    usable: usize,
}

impl<'a> Reader<'a> {
    pub fn open(bytes: &'a [u8]) -> Result<Self> {
        if bytes.len() < HEAD || &bytes[..16] != MAGIC {
            return Err(Error::new("not an SQLite database"));
        }
        let page = match be(bytes, 16, 2)? {
            1 => 65536,
            n => n as usize,
        };
        if page < 512 || !page.is_power_of_two() {
            return Err(bad("page size"));
        }
        if be(bytes, 56, 4)? > 1 {
            return Err(bad("text encoding is not UTF-8"));
        }
        let usable = page - bytes[20] as usize;
        Ok(Reader {
            bytes,
            page,
            usable,
        })
    }

    fn page(&self, n: u64) -> Result<&'a [u8]> {
        let at = (n as usize).checked_sub(1).ok_or_else(|| bad("page 0"))? * self.page;
        self.bytes
            .get(at..at + self.page)
            .ok_or_else(|| bad("page"))
    }

    fn payload(&self, page: &[u8], at: usize, size: usize) -> Result<Vec<u8>> {
        let u = self.usable;
        let x = u - 35;
        let local = if size <= x {
            size
        } else {
            let m = (u - 12) * 32 / 255 - 23;
            let k = m + (size - m) % (u - 4);
            if k <= x {
                k
            } else {
                m
            }
        };
        let mut out = page
            .get(at..at + local)
            .ok_or_else(|| bad("cell"))?
            .to_vec();
        let mut next = if local < size {
            be(page, at + local, 4)?
        } else {
            0
        };
        let mut hops = 0;
        while out.len() < size && next != 0 {
            let p = self.page(next)?;
            let take = (size - out.len()).min(u - 4);
            out.extend(p.get(4..4 + take).ok_or_else(|| bad("overflow"))?);
            next = be(p, 0, 4)?;
            hops += 1;
            if hops > self.bytes.len() / self.page {
                return Err(bad("overflow loop"));
            }
        }
        if out.len() != size {
            return Err(bad("payload"));
        }
        Ok(out)
    }

    fn scan(&self, root: u64, out: &mut Vec<(i64, Vec<u8>)>, depth: usize) -> Result<()> {
        if depth > 64 {
            return Err(bad("tree depth"));
        }
        let page = self.page(root)?;
        let base = if root == 1 { HEAD } else { 0 };
        let kind = page[base];
        let count = be(page, base + 3, 2)? as usize;
        let cells = base + if kind == 0x05 { 12 } else { 8 };
        for i in 0..count {
            let at = be(page, cells + 2 * i, 2)? as usize;
            match kind {
                0x0D => {
                    let (size, a) = get_varint(page, at)?;
                    let (rowid, b) = get_varint(page, at + a)?;
                    let data = self.payload(page, at + a + b, size as usize)?;
                    out.push((rowid, data));
                }
                0x05 => self.scan(be(page, at, 4)?, out, depth + 1)?,
                _ => return Err(bad("not a table page")),
            }
        }
        if kind == 0x05 {
            self.scan(be(page, base + 8, 4)?, out, depth + 1)?;
        }
        Ok(())
    }

    fn rows(&self, root: u64) -> Result<Vec<(i64, Vec<Value>)>> {
        let mut raw = Vec::new();
        self.scan(root, &mut raw, 0)?;
        raw.into_iter()
            .map(|(id, p)| Ok((id, decode_record(&p)?)))
            .collect()
    }

    pub fn table(&self, name: &str) -> Result<Option<Table>> {
        for (_, row) in self.rows(1)? {
            let text = |i: usize| row.get(i).and_then(Value::text).unwrap_or_default();
            if text(0) != "table" || !text(1).eq_ignore_ascii_case(name) {
                continue;
            }
            let root = row.get(3).and_then(Value::int).ok_or_else(|| bad("root"))?;
            let defs = columns(&text(4));
            let alias = defs.iter().position(|(_, a)| *a);
            let columns: Vec<String> = defs.into_iter().map(|(c, _)| c).collect();
            let mut rows = Vec::new();
            for (id, mut values) in self.rows(root as u64)? {
                values.resize(columns.len(), Value::Null);
                if let Some(i) = alias {
                    values[i] = Value::Int(id);
                }
                rows.push(values);
            }
            return Ok(Some(Table { columns, rows }));
        }
        Ok(None)
    }
}

fn unquote(word: &str) -> String {
    let w = word.trim();
    let pairs = [('`', '`'), ('[', ']'), ('"', '"'), ('\'', '\'')];
    for (a, b) in pairs {
        if let Some(inner) = w.strip_prefix(a).and_then(|w| w.strip_suffix(b)) {
            return inner.to_string();
        }
    }
    w.to_string()
}

fn first_word(def: &str) -> &str {
    let def = def.trim_start();
    let close = match def.chars().next() {
        Some('`') => Some('`'),
        Some('[') => Some(']'),
        Some('"') => Some('"'),
        _ => None,
    };
    match close {
        Some(c) => def[1..].find(c).map_or(def, |i| &def[..i + 2]),
        None => def.split_whitespace().next().unwrap_or(""),
    }
}

pub fn columns(sql: &str) -> Vec<(String, bool)> {
    let (Some(open), Some(close)) = (sql.find('('), sql.rfind(')')) else {
        return Vec::new();
    };
    let mut defs = Vec::new();
    let mut depth = 0;
    let mut start = open + 1;
    for (i, c) in sql[..close].char_indices().skip(open + 1) {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                defs.push(&sql[start..i]);
                start = i + 1;
            }
            _ => (),
        }
    }
    defs.push(&sql[start..close]);
    let keywords = ["PRIMARY", "UNIQUE", "CHECK", "FOREIGN", "CONSTRAINT"];
    defs.into_iter()
        .filter(|d| {
            let w = first_word(d).to_ascii_uppercase();
            !d.trim().is_empty() && !keywords.contains(&w.as_str())
        })
        .map(|d| {
            let upper = d.to_ascii_uppercase();
            let words: Vec<&str> = upper.split_whitespace().collect();
            let alias = words.get(1) == Some(&"INTEGER")
                && words.windows(2).any(|w| w == ["PRIMARY", "KEY"])
                && !upper.contains("DESC");
            (unquote(first_word(d)), alias)
        })
        .collect()
}

pub struct Object {
    pub kind: &'static str,
    pub name: String,
    pub table: String,
    pub sql: Option<String>,
    pub rows: Vec<(i64, Vec<Value>)>,
}

fn cmp_keys(a: &[Value], b: &[Value]) -> Ordering {
    for (x, y) in a.iter().zip(b) {
        match x.cmp(y) {
            Ordering::Equal => continue,
            o => return o,
        }
    }
    a.len().cmp(&b.len())
}

pub fn index_keys(rows: &[(i64, Vec<Value>)], columns: &[usize]) -> Result<Vec<(i64, Vec<Value>)>> {
    let mut keys: Vec<Vec<Value>> = rows
        .iter()
        .map(|(id, r)| {
            let mut k: Vec<Value> = columns.iter().map(|c| r[*c].clone()).collect();
            k.push(Value::Int(*id));
            k
        })
        .collect();
    keys.sort_by(|a, b| cmp_keys(a, b));
    for w in keys.windows(2) {
        let n = w[0].len() - 1;
        if cmp_keys(&w[0][..n], &w[1][..n]) == Ordering::Equal {
            return Err(Error::new("duplicate key in a unique index"));
        }
    }
    Ok(keys.into_iter().map(|k| (0, k)).collect())
}

struct Pages {
    pages: Vec<Vec<u8>>,
}

fn base(number: usize) -> usize {
    if number == 1 {
        HEAD
    } else {
        0
    }
}

fn fits(number: usize, interior: bool, cells: &[Vec<u8>], next: &[u8]) -> bool {
    let head = if interior { 12 } else { 8 };
    let used: usize = cells.iter().map(|c| c.len() + 2).sum();
    base(number) + head + used + next.len() + 2 <= PAGE
}

impl Pages {
    fn alloc(&mut self) -> usize {
        self.pages.push(vec![0; PAGE]);
        self.pages.len()
    }

    fn put(&mut self, number: usize, kind: u8, cells: &[Vec<u8>], right: Option<usize>) {
        let page = &mut self.pages[number - 1];
        let at = base(number);
        let head = if right.is_some() { 12 } else { 8 };
        let mut end = PAGE;
        let mut ptr = at + head;
        for cell in cells {
            end -= cell.len();
            page[end..end + cell.len()].copy_from_slice(cell);
            page[ptr..ptr + 2].copy_from_slice(&(end as u16).to_be_bytes());
            ptr += 2;
        }
        page[at] = kind;
        page[at + 3..at + 5].copy_from_slice(&(cells.len() as u16).to_be_bytes());
        page[at + 5..at + 7].copy_from_slice(&(end as u16).to_be_bytes());
        if let Some(r) = right {
            page[at + 8..at + 12].copy_from_slice(&(r as u32).to_be_bytes());
        }
    }

    fn tree(&mut self, cells: Vec<Vec<u8>>, index: bool, root: Option<usize>) -> Result<usize> {
        let (leaf, inner) = if index { (0x0A, 0x02) } else { (0x0D, 0x05) };
        let limit = if index {
            (PAGE - 12) * 64 / 255 - 23
        } else {
            PAGE - 35
        };
        if cells.iter().any(|c| c.len() > limit) {
            return Err(Error::new("record is too large"));
        }
        let target = root.unwrap_or(2);
        let whole = cells.iter().map(|c| c.len() + 2).sum::<usize>();
        if base(target) + 8 + whole <= PAGE {
            let number = root.unwrap_or_else(|| self.alloc());
            self.put(number, leaf, &cells, None);
            return Ok(number);
        }
        let mut children = Vec::new();
        let mut keys: Vec<Vec<u8>> = Vec::new();
        let mut chunk: Vec<Vec<u8>> = Vec::new();
        for cell in cells {
            if !chunk.is_empty() && !fits(2, false, &chunk, &cell) {
                let key = if index {
                    chunk.pop().expect("chunk")
                } else {
                    let last = chunk.last().expect("chunk");
                    let (_, a) = get_varint(last, 0)?;
                    let mut k = Vec::new();
                    put_varint(&mut k, get_varint(last, a)?.0);
                    k
                };
                let number = self.alloc();
                self.put(number, leaf, &chunk, None);
                children.push(number);
                keys.push(key);
                chunk.clear();
            }
            chunk.push(cell);
        }
        let number = self.alloc();
        self.put(number, leaf, &chunk, None);
        children.push(number);
        self.interior(children, keys, inner, root)
    }

    fn interior(
        &mut self,
        children: Vec<usize>,
        keys: Vec<Vec<u8>>,
        kind: u8,
        root: Option<usize>,
    ) -> Result<usize> {
        let cell = |child: usize, key: &[u8]| {
            let mut c = (child as u32).to_be_bytes().to_vec();
            c.extend(key);
            c
        };
        let cells: Vec<Vec<u8>> = keys
            .iter()
            .zip(&children)
            .map(|(k, c)| cell(*c, k))
            .collect();
        let last = *children.last().expect("children");
        let target = root.unwrap_or(2);
        let whole = cells.iter().map(|c| c.len() + 2).sum::<usize>();
        if base(target) + 12 + whole <= PAGE {
            let number = root.unwrap_or_else(|| self.alloc());
            self.put(number, kind, &cells, Some(last));
            return Ok(number);
        }
        let mut next_children = Vec::new();
        let mut next_keys = Vec::new();
        let mut chunk: Vec<Vec<u8>> = Vec::new();
        for (i, c) in cells.iter().enumerate() {
            if !chunk.is_empty() && !fits(2, true, &chunk, c) {
                let number = self.alloc();
                self.put(number, kind, &chunk, Some(children[i]));
                next_children.push(number);
                next_keys.push(keys[i].clone());
                chunk.clear();
                continue;
            }
            chunk.push(c.clone());
        }
        let number = self.alloc();
        self.put(number, kind, &chunk, Some(last));
        next_children.push(number);
        self.interior(next_children, next_keys, kind, root)
    }
}

fn table_cell(rowid: i64, record: &[u8]) -> Vec<u8> {
    let mut c = Vec::new();
    put_varint(&mut c, record.len() as i64);
    put_varint(&mut c, rowid);
    c.extend(record);
    c
}

fn index_cell(record: &[u8]) -> Vec<u8> {
    let mut c = Vec::new();
    put_varint(&mut c, record.len() as i64);
    c.extend(record);
    c
}

pub fn write(objects: &[Object]) -> Result<Vec<u8>> {
    let mut p = Pages {
        pages: vec![vec![0; PAGE]],
    };
    let mut master = Vec::new();
    for (i, o) in objects.iter().enumerate() {
        let index = o.kind == "index";
        let cells = o.rows.iter().map(|(id, r)| {
            let record = encode_record(r);
            if index {
                index_cell(&record)
            } else {
                table_cell(*id, &record)
            }
        });
        let root = p.tree(cells.collect(), index, None)?;
        let sql = o.sql.clone().map_or(Value::Null, Value::Text);
        let row = vec![
            Value::Text(o.kind.into()),
            Value::Text(o.name.clone()),
            Value::Text(o.table.clone()),
            Value::Int(root as i64),
            sql,
        ];
        master.push(table_cell(i as i64 + 1, &encode_record(&row)));
    }
    p.tree(master, false, Some(1))?;
    let count = p.pages.len() as u32;
    let head = &mut p.pages[0][..HEAD];
    head[..16].copy_from_slice(MAGIC);
    head[16..18].copy_from_slice(&(PAGE as u16).to_be_bytes());
    head[18] = 1;
    head[19] = 1;
    head[21] = 64;
    head[22] = 32;
    head[23] = 32;
    head[24..28].copy_from_slice(&1u32.to_be_bytes());
    head[28..32].copy_from_slice(&count.to_be_bytes());
    head[40..44].copy_from_slice(&1u32.to_be_bytes());
    head[44..48].copy_from_slice(&4u32.to_be_bytes());
    head[56..60].copy_from_slice(&1u32.to_be_bytes());
    head[92..96].copy_from_slice(&1u32.to_be_bytes());
    head[96..100].copy_from_slice(&3_045_000u32.to_be_bytes());
    Ok(p.pages.concat())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varints() {
        for v in [
            0,
            1,
            127,
            128,
            16383,
            16384,
            1 << 40,
            i64::MAX,
            -1,
            i64::MIN,
        ] {
            let mut out = Vec::new();
            put_varint(&mut out, v);
            assert_eq!(get_varint(&out, 0).unwrap(), (v, out.len()));
        }
    }

    #[test]
    fn records() {
        let values = vec![
            Value::Null,
            Value::Int(0),
            Value::Int(1),
            Value::Int(-300),
            Value::Int(1 << 40),
            Value::Real(7.5),
            Value::Text("JA1ZLO".into()),
            Value::Blob(vec![1, 2]),
        ];
        assert_eq!(decode_record(&encode_record(&values)).unwrap(), values);
    }

    #[test]
    fn column_names() {
        let sql =
            "CREATE TABLE `T` (`A` INTEGER PRIMARY KEY, [B] TEXT(16), c REAL, PRIMARY KEY (`A`))";
        let cols = columns(sql);
        assert_eq!(
            cols,
            [("A".into(), true), ("B".into(), false), ("c".into(), false)]
        );
    }

    #[test]
    fn large_tables_round_trip() {
        let sql = "CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT)";
        let rows: Vec<(i64, Vec<Value>)> = (1..=5000)
            .map(|i| (i, vec![Value::Null, Value::Text(format!("call{i:05}"))]))
            .collect();
        let keys = index_keys(&rows, &[1]).unwrap();
        let objects = [
            Object {
                kind: "table",
                name: "t".into(),
                table: "t".into(),
                sql: Some(sql.into()),
                rows,
            },
            Object {
                kind: "index",
                name: "i".into(),
                table: "t".into(),
                sql: Some("CREATE INDEX i ON t(name)".into()),
                rows: keys,
            },
        ];
        let bytes = write(&objects).unwrap();
        let table = Reader::open(&bytes).unwrap().table("t").unwrap().unwrap();
        assert_eq!(table.rows.len(), 5000);
        assert_eq!(table.rows[4999][0], Value::Int(5000));
        assert_eq!(table.rows[4999][1], Value::Text("call05000".into()));
    }
}
