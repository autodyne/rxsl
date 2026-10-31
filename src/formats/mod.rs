pub mod adis;
pub mod adxs;
pub mod cbin;
pub mod cqww;
pub mod ctxt;
pub mod hbin;
pub mod jarl;
pub mod jswl;
pub mod qxml;
pub mod zall;
pub mod zdos;

mod xml;

use crate::model::{Item, QName};

pub(crate) fn some<'a>(item: &'a Item, local: &str) -> &'a str {
    item.attrs.qxsl(local).unwrap_or("")
}

pub(crate) fn set_text(node: &mut crate::Node, local: &str, text: &str) {
    if !text.is_empty() {
        node.set(QName::qxsl(local), text);
    }
}

pub(crate) fn set_rstq(node: &mut crate::Node, text: &str) -> crate::Result<()> {
    if !text.is_empty() {
        let n: i32 = text
            .parse()
            .map_err(|_| crate::Error::new(format!("malformed RST: {text}")))?;
        node.set(QName::qxsl("rstq"), n.to_string());
    }
    Ok(())
}

pub(crate) fn set_band(node: &mut crate::Node, text: &str) -> crate::Result<()> {
    if !text.is_empty() {
        node.set(QName::qxsl("band"), crate::Band::parse(text)?.khz());
    }
    Ok(())
}

pub(crate) fn zlog_band(item: &Item) -> String {
    match item
        .attrs
        .qxsl("band")
        .and_then(|b| crate::Band::parse(b).ok())
    {
        Some(b) => zlog_label(b),
        None => String::new(),
    }
}

const SHF_NOMINAL: u64 = 10_000_000_000;

fn zlog_label(b: crate::Band) -> String {
    match b.0 {
        SHF_NOMINAL => "10.1G".into(),
        hz if hz >= SHF_NOMINAL => format!("{}G", crate::Band(hz / 1000).mhz()),
        _ => b.mhz(),
    }
}

pub(crate) fn zlog_band_text(text: &str) -> String {
    match text.strip_suffix('G') {
        Some("10.1") => "10GHz".into(),
        Some(g) => format!("{g}GHz"),
        None if text.ends_with(|c: char| c.is_ascii_digit()) => format!("{text}MHz"),
        None => text.to_string(),
    }
}

pub(crate) fn md_hm(
    month: &str,
    day: &str,
    hhmm: &str,
    year: i32,
    offset: i32,
) -> crate::Result<crate::Time> {
    let bad = || crate::Error::new(format!("malformed time: {month} {day} {hhmm}"));
    let m: u32 = month.trim().parse().map_err(|_| bad())?;
    let d: u32 = day.trim().parse().map_err(|_| bad())?;
    if hhmm.len() != 4 || !hhmm.is_ascii() {
        return Err(bad());
    }
    let h: u32 = hhmm[..2].parse().map_err(|_| bad())?;
    let mi: u32 = hhmm[2..].parse().map_err(|_| bad())?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) || h > 23 || mi > 59 {
        return Err(bad());
    }
    Ok(crate::Time::new((year, m, d), (h, mi, 0), offset))
}

pub(crate) fn local_time(item: &Item, offset: i32) -> Option<crate::Time> {
    item.attrs
        .qxsl("time")
        .and_then(|t| crate::Time::parse_iso(t).ok())
        .map(|t| t.at_offset(offset))
}

const ZLOG_BANDS: [&str; 22] = [
    "1.9", "3.5", "7", "10", "14", "18", "21", "24", "28", "50", "144", "430", "1200", "2400",
    "5600", "10.1G", "10.4G", "24G", "47G", "77G", "135G", "248G",
];

const ZLOG_FLAGS: [&str; 4] = ["PSE QSL", "NO QSL", "*", "-DUPE-"];

pub(crate) struct ZLogRow {
    pub mul1: String,
    pub mul2: String,
    pub band: String,
    pub points: String,
    pub memo: String,
}

pub(crate) fn zlog_row(item: &Item) -> ZLogRow {
    let exact = item
        .attrs
        .qxsl("band")
        .and_then(|b| crate::Band::parse(b).ok());
    let nominal = exact.and_then(|b| match ZLOG_BANDS.contains(&zlog_label(b).as_str()) {
        true => Some(b),
        false => crate::conv::nominal_band(&b.khz()).and_then(|b| crate::Band::parse(&b).ok()),
    });
    let band = nominal.or(exact).map(zlog_label).unwrap_or_default();
    let new = item.attrs.qxsl("newmult") != Some("false");
    let mul = |k: &str| match some(item, k) {
        m if new && !m.is_empty() => Some(m.to_string()),
        _ => None,
    };
    let (flags, note) = zlog_flags(some(item, "note"));
    let mut memo = String::new();
    let add = |memo: &mut String, s: &str| {
        if !memo.is_empty() {
            memo.push(' ');
        }
        memo.push_str(s);
    };
    for f in flags {
        add(&mut memo, f);
    }
    if let (Some(e), Some(n)) = (exact, nominal) {
        if e != n {
            add(&mut memo, &format!("({}.{})", e.0 / 1000, e.0 % 1000 / 100));
        }
    }
    add(&mut memo, note);
    ZLogRow {
        mul1: mul("mul1").unwrap_or_else(|| "-".into()),
        mul2: mul("mul2").unwrap_or_else(|| "-".into()),
        band,
        points: item.attrs.qxsl("points").unwrap_or("1").to_string(),
        memo,
    }
}

pub(crate) fn set_zlog_mults(item: &mut Item, mul1: &str, mul2: &str) {
    for (k, m) in [("mul1", mul1), ("mul2", mul2)] {
        if m != "-" {
            set_text(&mut item.attrs, k, m);
        }
    }
    match (mul1, mul2) {
        ("-", "-") => item.attrs.set(QName::qxsl("newmult"), "false"),
        ("", "") | ("-", "") => (),
        _ => item.attrs.set(QName::qxsl("newmult"), "true"),
    }
}

pub(crate) fn set_zlog_rest(item: &mut Item, points: &str, rest: &str) {
    if let Ok(p) = points.parse::<i64>() {
        item.attrs.set(QName::qxsl("points"), p.to_string());
    }
    let mut tail = rest;
    if let Some((name, memo)) = rest.strip_prefix("%%").and_then(|r| r.split_once("%%")) {
        item.attrs.set(QName::qxsl("name"), name.trim());
        tail = memo;
    }
    let tail = tail.trim();
    let (_, text) = zlog_flags(tail);
    let head = &tail[..tail.len() - text.len()];
    let band = item
        .attrs
        .qxsl("band")
        .and_then(|b| crate::Band::parse(b).ok());
    let word = text.split(' ').next().unwrap_or("");
    let freq = word
        .strip_prefix('(')
        .and_then(|w| w.strip_suffix(')'))
        .and_then(|w| crate::Band::parse(w).ok())
        .filter(|f| band.is_some_and(|b| crate::conv::nominal_band(&f.khz()) == Some(b.khz())));
    let note = match freq {
        Some(f) => {
            item.attrs.set(QName::qxsl("band"), f.khz());
            format!("{head}{}", text[word.len()..].trim_start())
        }
        None => tail.to_string(),
    };
    set_text(&mut item.attrs, "note", note.trim());
}

fn zlog_flags(mut text: &str) -> (Vec<&'static str>, &str) {
    let mut flags = Vec::new();
    'flags: loop {
        for f in ZLOG_FLAGS {
            if let Some(rest) = text.strip_prefix(f) {
                if rest.is_empty() || rest.starts_with(' ') {
                    flags.push(f);
                    text = rest.trim_start();
                    continue 'flags;
                }
            }
        }
        return (flags, text);
    }
}
