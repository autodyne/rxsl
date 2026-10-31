use crate::model::{Item, QName, ADIF};
use crate::text::{decode_sjis, NEWLINE};
use crate::{Error, Format, Result};

pub struct Adis;

impl Format for Adis {
    fn kind(&self) -> &'static str {
        "adis"
    }

    fn name(&self) -> &'static str {
        "ADIF (ADI)"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["adi"]
    }

    fn decode(&self, bytes: &[u8]) -> Result<Vec<Item>> {
        let utf8 = std::str::from_utf8(bytes).is_ok();
        let upper = bytes.to_ascii_uppercase();
        let find =
            |from: usize, pat: &[u8]| upper[from..].windows(pat.len()).position(|w| w == pat);
        let mut pos = 0;
        let mut valid = false;
        if !bytes.starts_with(b"<") {
            match find(0, b"<EOH>") {
                Some(p) => {
                    pos = p + 5;
                    valid = true;
                }
                None => pos = bytes.len(),
            }
        }
        let mut items = Vec::new();
        while let Some(p) = find(pos, b"<EOR>") {
            items.push(record_item(&bytes[pos..pos + p], utf8));
            pos += p + 5;
            valid = true;
        }
        if !valid {
            return Err(Error::new("no header/records detected"));
        }
        Ok(items)
    }

    fn encode(&self, items: &[Item]) -> Result<Vec<u8>> {
        let mut out = format!("ADIF\n<adif_ver:5>3.0.0\n<programid:4>qxsl\n<EOH>{NEWLINE}");
        for item in items {
            for (k, v) in item.attrs.iter().filter(|(k, _)| k.space == ADIF) {
                out.push_str(&format!("<{}:{}>{v}", k.local, v.len()));
            }
            out.push_str("<EOR>");
            out.push_str(NEWLINE);
        }
        Ok(out.into_bytes())
    }
}

fn record_item(record: &[u8], utf8: bool) -> Item {
    let mut item = Item::default();
    let mut i = 0;
    while i < record.len() {
        if record[i] != b'<' {
            i += 1;
            continue;
        }
        match tag(record, i + 1) {
            Some((name, len, end)) => {
                let raw = &record[end..(end + len).min(record.len())];
                let value = if utf8 {
                    String::from_utf8_lossy(raw).into_owned()
                } else {
                    decode_sjis(raw)
                };
                item.attrs.set(QName::new(ADIF, name.to_uppercase()), value);
                i = end;
            }
            None => i += 1,
        }
    }
    item
}

fn tag(c: &[u8], mut i: usize) -> Option<(String, usize, usize)> {
    let word = |ch: u8| ch.is_ascii_alphanumeric() || ch == b'_';
    let start = i;
    while i < c.len() && word(c[i]) {
        i += 1;
    }
    if i == start || c.get(i) != Some(&b':') {
        return None;
    }
    let name = String::from_utf8_lossy(&c[start..i]).into_owned();
    i += 1;
    let ds = i;
    while i < c.len() && c[i].is_ascii_digit() {
        i += 1;
    }
    if i == ds {
        return None;
    }
    let len: usize = std::str::from_utf8(&c[ds..i]).ok()?.parse().ok()?;
    if c.get(i) == Some(&b':') {
        if !c.get(i + 1).is_some_and(|&ch| word(ch)) || c.get(i + 2) != Some(&b'>') {
            return None;
        }
        i += 2;
    }
    if c.get(i) != Some(&b'>') {
        return None;
    }
    Some((name, len, i + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_header_and_types() {
        let src = b"ADIF export\n<EOH>\n<CALL:6>JA1ZLO<freq:6:N>14.025\n<EOR>\n";
        let items = Adis.decode(src).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].attrs.adif("CALL"), Some("JA1ZLO"));
        assert_eq!(items[0].attrs.adif("FREQ"), Some("14.025"));
        assert!(Adis.decode(b"nothing here").is_err());
    }
}
