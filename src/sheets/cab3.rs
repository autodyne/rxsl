use crate::sheets::{Field, Sheet, SheetFormat};
use crate::text::{decode_ascii, encode_ascii, lines, NEWLINE};
use crate::{Error, Result};

pub const VERSION: &str = "3.0";

pub const TABLE: [&str; 4] = ["QSO", "X-QSO", "QTC", "X-QTC"];

pub const TAGS: [&str; 27] = [
    "CALLSIGN",
    "CONTEST",
    "CATEGORY-ASSISTED",
    "CATEGORY-BAND",
    "CATEGORY-MODE",
    "CATEGORY-OPERATOR",
    "CATEGORY-POWER",
    "CATEGORY-STATION",
    "CATEGORY-TIME",
    "CATEGORY-TRANSMITTER",
    "CATEGORY-OVERLAY",
    "CERTIFICATE",
    "CLAIMED-SCORE",
    "CLUB",
    "CREATED-BY",
    "EMAIL",
    "GRID-LOCATOR",
    "LOCATION",
    "NAME",
    "ADDRESS",
    "ADDRESS-CITY",
    "ADDRESS-STATE-PROVINCE",
    "ADDRESS-POSTALCODE",
    "ADDRESS-COUNTRY",
    "OPERATORS",
    "OFFTIME",
    "SOAPBOX",
];

pub fn header(values: &[(&str, &str)]) -> Vec<Field> {
    let get = |t: &str| values.iter().find(|(k, _)| k.eq_ignore_ascii_case(t));
    let mut out: Vec<Field> = TAGS
        .iter()
        .map(|t| Field::new(*t, get(t).map_or("", |(_, v)| *v)))
        .collect();
    for (k, v) in values {
        if !TAGS.iter().any(|t| t.eq_ignore_ascii_case(k)) {
            out.push(Field::new(*k, *v));
        }
    }
    out
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Cab3Sheet {
    pub utf8: bool,
}

fn is_table(line: &str) -> bool {
    line.split_once(':')
        .is_some_and(|(k, _)| TABLE.iter().any(|t| t.eq_ignore_ascii_case(k.trim())))
}

impl SheetFormat for Cab3Sheet {
    fn kind(&self) -> &'static str {
        "cab3"
    }

    fn name(&self) -> &'static str {
        "Cabrillo3"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["cbr", "log"]
    }

    fn decode(&self, bytes: &[u8]) -> Result<Sheet> {
        let text = match std::str::from_utf8(bytes) {
            Ok(t) => t.to_string(),
            Err(_) => decode_ascii(bytes),
        };
        let mut rows = lines(&text).into_iter().skip_while(|l| l.trim().is_empty());
        let head = rows.next().unwrap_or("");
        let version = head
            .strip_prefix("START-OF-LOG:")
            .ok_or_else(|| Error::new("'START-OF-LOG:' is not found"))?;
        let mut sheet = Sheet {
            version: version.trim().to_string(),
            ..Sheet::default()
        };
        let mut table = String::new();
        for line in rows {
            if line.trim_end() == "END-OF-LOG:" {
                break;
            }
            if is_table(line) {
                table.push_str(line);
                table.push_str(NEWLINE);
            } else if let Some((k, v)) = line.split_once(':') {
                let v = v.strip_prefix(' ').unwrap_or(v);
                sheet.push(Field::new(k.trim(), v.trim_end()));
            }
        }
        sheet.table = table.into_bytes();
        Ok(sheet)
    }

    fn encode(&self, sheet: &Sheet) -> Result<Vec<u8>> {
        let version = match sheet.version.as_str() {
            "" => VERSION,
            v => v,
        };
        let mut out = format!("START-OF-LOG: {version}{NEWLINE}");
        for f in &sheet.fields {
            for v in f.value.split('\n').map(|v| v.trim_end_matches('\r')) {
                out.push_str(&format!("{}: {v}{NEWLINE}", f.key));
            }
        }
        for line in lines(&String::from_utf8_lossy(&sheet.table)) {
            if is_table(line) {
                out.push_str(line);
                out.push_str(NEWLINE);
            }
        }
        out.push_str(&format!("END-OF-LOG:{NEWLINE}"));
        Ok(match self.utf8 {
            true => out.into_bytes(),
            false => encode_ascii(&out),
        })
    }
}
