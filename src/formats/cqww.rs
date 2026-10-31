use crate::formats::{set_rstq, set_text, some};
use crate::model::{Item, QName};
use crate::text::{columns, decode_ascii, encode_ascii, fill_head, fill_tail, lines, NEWLINE};
use crate::{Band, Error, Format, Result, Time};

pub struct Cqww;

const BANDS: &[(&str, u64)] = &[
    ("1800", 1_800),
    ("3500", 3_500),
    ("7000", 7_000),
    ("14000", 14_000),
    ("21000", 21_000),
    ("28000", 28_000),
    ("50", 50_000),
    ("70", 70_000),
    ("144", 144_000),
    ("222", 222_000),
    ("432", 432_000),
    ("902", 902_000),
    ("1.2G", 1_200_000),
    ("2.3G", 2_300_000),
    ("3.4G", 3_400_000),
    ("5.7G", 5_700_000),
    ("10G", 10_000_000),
    ("24G", 24_000_000),
    ("47G", 47_000_000),
    ("75G", 75_000_000),
    // Cabrillo v3 says 122G; QxSL's 123G is still read.
    ("122G", 123_000_000),
    ("123G", 123_000_000),
    ("134G", 134_000_000),
    ("241G", 241_000_000),
];

impl Format for Cqww {
    fn kind(&self) -> &'static str {
        "cqww"
    }

    fn name(&self) -> &'static str {
        "Cabrillo (CQWW)"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["cbr"]
    }

    fn decode(&self, bytes: &[u8]) -> Result<Vec<Item>> {
        let text = decode_ascii(bytes);
        let mut items = Vec::new();
        for line in lines(&text) {
            let (line, invalid) = match line.strip_prefix("X-") {
                Some(rest) if rest.starts_with("QSO:") => (rest, true),
                _ if line.starts_with("QSO:") => (line, false),
                _ => continue,
            };
            let v = fields(line)?;
            let mut item = Item::default();
            if invalid {
                item.attrs.set(QName::qxsl("invalid"), "true");
            }
            if !v[0].is_empty() {
                let band = match BANDS.iter().find(|(t, _)| *t == v[0]) {
                    Some((_, k)) => Band::from_khz(*k).khz(),
                    None => Band::parse(&v[0])?.khz(),
                };
                item.attrs.set(QName::qxsl("band"), band);
            }
            set_text(&mut item.attrs, "mode", &v[1]);
            if !v[2].is_empty() {
                item.attrs.set(QName::qxsl("time"), time(&v[2])?.to_iso());
            }
            if !v[3].chars().all(|c| c == '*') {
                set_text(&mut item.sent, "call", &v[3]);
            }
            set_rstq(&mut item.sent, &v[4])?;
            set_text(&mut item.sent, "code", &v[5]);
            set_text(&mut item.rcvd, "call", &v[6]);
            set_rstq(&mut item.rcvd, &v[7])?;
            set_text(&mut item.rcvd, "code", &v[8]);
            items.push(item);
        }
        Ok(items)
    }

    fn encode(&self, items: &[Item]) -> Result<Vec<u8>> {
        let mycall = items.iter().find_map(|i| i.sent.qxsl("call")).unwrap_or("");
        let mut out = format!("START-OF-LOG: 3.0{NEWLINE}CALLSIGN: {mycall}{NEWLINE}");
        for tag in TAGS {
            out.push_str(&format!("{tag}: {NEWLINE}"));
        }
        for item in items {
            if item.attrs.qxsl("invalid") == Some("true") {
                out.push_str("X-");
            }
            let t = crate::formats::local_time(item, 0)
                .map(|t| {
                    format!(
                        "{:04}-{:02}-{:02} {:02}{:02}",
                        t.year, t.month, t.day, t.hour, t.minute
                    )
                })
                .unwrap_or_else(|| " ".repeat(15));
            let cols = [
                "QSO:".to_string(),
                fill_head(&band(item), 5),
                fill_tail(some(item, "mode"), 2),
                t,
                fill_tail(item.sent.qxsl("call").unwrap_or(""), 13),
                fill_head(item.sent.qxsl("rstq").unwrap_or(""), 3),
                fill_tail(item.sent.qxsl("code").unwrap_or(""), 6),
                fill_tail(item.rcvd.qxsl("call").unwrap_or(""), 13),
                fill_head(item.rcvd.qxsl("rstq").unwrap_or(""), 3),
                fill_tail(item.rcvd.qxsl("code").unwrap_or(""), 6),
                "0".to_string(),
            ];
            out.push_str(&cols.join(" "));
            out.push_str(NEWLINE);
        }
        out.push_str(&format!("END-OF-LOG:{NEWLINE}"));
        Ok(encode_ascii(&out))
    }
}

const TAGS: [&str; 26] = [
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

fn band(item: &Item) -> String {
    let Some(b) = item.attrs.qxsl("band").and_then(|b| Band::parse(b).ok()) else {
        return String::new();
    };
    match BANDS.iter().find(|(_, k)| Band::from_khz(*k) == b) {
        Some((t, _)) => t.to_string(),
        None => b.khz(),
    }
}

fn fields(line: &str) -> Result<Vec<String>> {
    let fixed = columns(line, &[5, 11, 14, 30, 44, 48, 55, 69, 73, 80]);
    let rst = |s: &str| s.parse::<u32>().is_ok();
    if let Ok(v) = &fixed {
        if !v[6].is_empty() && [&v[4], &v[7]].iter().all(|r| r.is_empty() || rst(r)) {
            return fixed;
        }
    }
    let mut t: Vec<&str> = line.split_whitespace().skip(1).collect();
    let mycall = t.len() > 4 && !rst(t[4]);
    if (t.len() == 9 || t.len() == 10) && !mycall {
        t.insert(4, "");
    }
    if t.len() == 10 || t.len() == 11 {
        let mut v: Vec<String> = t[..10].iter().map(|s| s.to_string()).collect();
        let time = v.remove(3);
        v[2] = format!("{} {time}", v[2]);
        return Ok(v);
    }
    fixed
}

fn time(text: &str) -> Result<Time> {
    let bad = || Error::new(format!("malformed time: {text}"));
    let (d, hm) = text.split_once(' ').ok_or_else(bad)?;
    if hm.len() != 4 || !hm.is_ascii() {
        return Err(bad());
    }
    Time::parse_iso(&format!("{d}T{}:{}Z", &hm[..2], &hm[2..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(own: Option<&str>) -> Item {
        let mut item = Item::default();
        item.attrs.set(QName::qxsl("band"), "14000");
        item.attrs.set(QName::qxsl("mode"), "CW");
        item.rcvd.set(QName::qxsl("call"), "QV1DOK");
        if let Some(call) = own {
            item.sent.set(QName::qxsl("call"), call);
        }
        item
    }

    fn line(item: Item) -> String {
        let text = String::from_utf8(Cqww.encode(&[item]).unwrap()).unwrap();
        let line = text.lines().find(|l| l.starts_with("QSO:")).unwrap();
        format!("{line}{NEWLINE}")
    }

    #[test]
    fn own_callsign_is_written_and_read_back() {
        let text = line(item(Some("JA1ZLO")));
        assert_eq!(&text[30..44], "JA1ZLO        ");
        let back = Cqww.decode(text.as_bytes()).unwrap();
        assert_eq!(back[0].sent.qxsl("call"), Some("JA1ZLO"));
    }

    #[test]
    fn without_own_callsign_spaces_as_zlog() {
        let text = line(item(None));
        assert_eq!(&text[30..44], "              ");
        let back = Cqww.decode(text.as_bytes()).unwrap();
        assert_eq!(back[0].sent.qxsl("call"), None);
    }

    #[test]
    fn single_spaced_lines_of_other_loggers_are_read() {
        let text = "QSO: 7000 RY 2017-06-04 0015 JA1ZLO 599 100110 QV1DOK 599 120103 0\r\n\
                    X-QSO: 14000 CW 2017-06-04 0016 ***** 599 1 QV1DOK 599 2\r\n";
        let v = Cqww.decode(text.as_bytes()).unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].attrs.qxsl("mode"), Some("RY"));
        assert_eq!(v[0].attrs.qxsl("band"), Some("7000"));
        assert_eq!(v[0].sent.qxsl("call"), Some("JA1ZLO"));
        assert_eq!(v[0].sent.qxsl("code"), Some("100110"));
        assert_eq!(v[0].rcvd.qxsl("call"), Some("QV1DOK"));
        assert_eq!(v[0].rcvd.qxsl("code"), Some("120103"));
        assert_eq!(v[1].attrs.qxsl("invalid"), Some("true"));
        assert_eq!(v[1].sent.qxsl("call"), None);
        assert_eq!(v[1].rcvd.qxsl("code"), Some("2"));
    }

    #[test]
    fn long_fields_are_written_whole() {
        let mut it = item(Some("JA1ZLO/2/QRP/X"));
        it.rcvd.set(QName::qxsl("call"), "QV1DOK/MM/ABCDE");
        it.rcvd.set(QName::qxsl("code"), "1234567890");
        let text = line(it);
        assert!(text.contains(" JA1ZLO/2/QRP/X "), "{text}");
        assert!(text.contains(" QV1DOK/MM/ABCDE "), "{text}");
        assert!(text.contains(" 1234567890 "), "{text}");
    }
}
