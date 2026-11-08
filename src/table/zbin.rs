use crate::model::{Item, Node, QName};
use crate::text::{decode_sjis, sjis_cut};
use crate::{Band, Error, Format, Result, Time};

pub struct ZBin;

pub const BANDS: &[u64] = &[
    1900, 3500, 7000, 10000, 14000, 18000, 21000, 24000, 28000, 50000, 144000, 430000, 1200000,
    2400000, 5600000, 10000000,
];

pub const MODES: &[&str] = &["CW", "SSB", "FM", "AM", "RTTY", "Other"];

pub const WATTS: &[&str] = &["P", "L", "M", "H"];

pub(crate) const RECORD: usize = 256;

const HEAD: usize = 256;
const ZLOX: &[u8] = b"ZLOX";
const USE_UTC: i16 = 0x7FFF;
const USE_JST: i16 = 0;
const DAY_MS: i64 = 86_400_000;

impl Format for ZBin {
    fn kind(&self) -> &'static str {
        "zbin"
    }

    fn name(&self) -> &'static str {
        "zLog (ZLO)"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["zlo"]
    }

    fn decode(&self, bytes: &[u8]) -> Result<Vec<Item>> {
        if bytes.len() < HEAD || !(bytes.len() - HEAD).is_multiple_of(RECORD) {
            return Err(Error::new("malformed ZLO sequence"));
        }
        if &bytes[..4] == ZLOX {
            return Err(Error::new("TQSODataEx detected"));
        }
        let offset = zone(&bytes[0x54..0x56]);
        let records = bytes[HEAD..].chunks(RECORD);
        Ok(records.map(|r| record(r, offset, MODES)).collect())
    }

    fn encode(&self, items: &[Item]) -> Result<Vec<u8>> {
        let mut out = vec![0; 0x54];
        out.extend(header_zone());
        out.resize(HEAD, 0);
        for (id, item) in items.iter().enumerate() {
            out.extend(write(item, id, MODES));
        }
        Ok(out)
    }
}

pub(crate) fn zone(bytes: &[u8]) -> i32 {
    match i16::from_le_bytes([bytes[0], bytes[1]]) {
        USE_UTC => 0,
        USE_JST => crate::value::JST,
        bias => -(bias as i32),
    }
}

pub(crate) fn header_zone() -> [u8; 2] {
    match crate::value::local_offset() {
        0 => USE_UTC,
        offset => -offset as i16,
    }
    .to_le_bytes()
}

fn epoch(offset: i32) -> i64 {
    Time::new((1899, 12, 30), (0, 0, 0), offset).unix() * 1000
}

fn time(bytes: &[u8], offset: i32) -> Time {
    let d = f64::from_le_bytes(bytes.try_into().expect("8 bytes"));
    let ms = (d.abs().fract() * DAY_MS as f64).round() as i64;
    let at = epoch(offset) + d.trunc() as i64 * DAY_MS + ms;
    Time::from_unix((at + 500).div_euclid(1000), offset)
}

fn days(time: &Time) -> f64 {
    let m = time.unix() * 1000 - epoch(crate::value::local_offset());
    (m.abs() % DAY_MS) as f64 / DAY_MS as f64 + (m / DAY_MS) as f64
}

fn text(bytes: &[u8], max: usize) -> String {
    let len = (bytes[0] as usize).min(max);
    decode_sjis(&bytes[1..1 + len])
}

fn put(node: &mut Node, local: &str, value: String) {
    if !value.is_empty() {
        node.set(QName::qxsl(local), value);
    }
}

pub(crate) fn record(r: &[u8], offset: i32, modes: &[&str]) -> Item {
    let mut item = Item::default();
    let at = time(&r[0..8], offset);
    item.attrs.set(QName::qxsl("time"), at.to_iso());
    put(&mut item.rcvd, "call", text(&r[8..21], 12));
    put(&mut item.sent, "code", text(&r[21..52], 30));
    put(&mut item.rcvd, "code", text(&r[52..83], 30));
    let rst = |at: usize| i16::from_le_bytes([r[at], r[at + 1]]).to_string();
    item.sent.set(QName::qxsl("rstq"), rst(84));
    item.rcvd.set(QName::qxsl("rstq"), rst(86));
    if let Some(m) = modes.get(r[92] as usize) {
        item.attrs.set(QName::qxsl("mode"), *m);
    }
    if let Some(b) = BANDS.get(r[93] as usize) {
        item.attrs
            .set(QName::qxsl("band"), Band::from_khz(*b).khz());
    }
    if let Some(w) = WATTS.get(r[94] as usize) {
        item.sent.set(QName::qxsl("watt"), *w);
    }
    put(&mut item.attrs, "mul1", text(&r[95..126], 30));
    put(&mut item.attrs, "mul2", text(&r[126..157], 30));
    put(&mut item.attrs, "name", text(&r[160..175], 14));
    put(&mut item.attrs, "note", text(&r[175..240], 64));
    item
}

fn field(out: &mut Vec<u8>, value: Option<&str>, size: usize) {
    let bytes = sjis_cut(value.unwrap_or(""), size);
    out.push(bytes.len() as u8);
    out.extend(&bytes);
    out.resize(out.len() + size - bytes.len(), 0);
}

fn index<T: PartialEq + ?Sized>(list: &[&T], value: Option<&T>) -> Option<u8> {
    list.iter().position(|x| Some(*x) == value).map(|i| i as u8)
}

pub(crate) fn write(item: &Item, id: usize, modes: &[&str]) -> Vec<u8> {
    let mut out = Vec::with_capacity(RECORD);
    let at = item
        .attrs
        .qxsl("time")
        .and_then(|t| Time::parse_iso(t).ok());
    out.extend(at.map_or(0.0, |t| days(&t)).to_le_bytes());
    field(&mut out, item.rcvd.qxsl("call"), 12);
    field(&mut out, item.sent.qxsl("code"), 30);
    field(&mut out, item.rcvd.qxsl("code"), 30);
    out.push(0);
    let rst = |n: &Node| {
        n.qxsl("rstq")
            .and_then(|r| r.parse::<i16>().ok())
            .unwrap_or(0)
    };
    out.extend(rst(&item.sent).to_le_bytes());
    out.extend(rst(&item.rcvd).to_le_bytes());
    out.extend([0; 4]);
    let other = modes.iter().position(|m| *m == "Other").unwrap_or(0) as u8;
    out.push(index(modes, item.attrs.qxsl("mode")).unwrap_or(other));
    let band = item.attrs.qxsl("band").and_then(|b| Band::parse(b).ok());
    let band = BANDS.iter().position(|k| Some(Band::from_khz(*k)) == band);
    out.push(band.map_or(0xFF, |i| i as u8));
    out.push(index(WATTS, item.sent.qxsl("watt")).unwrap_or(0));
    field(&mut out, item.attrs.qxsl("mul1"), 30);
    field(&mut out, item.attrs.qxsl("mul2"), 30);
    out.extend([0; 3]);
    field(&mut out, item.attrs.qxsl("name"), 14);
    field(&mut out, item.attrs.qxsl("note"), 64);
    out.extend([0; 12]);
    out.extend(((id * 100) as i32).to_le_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_time_round_trip() {
        let t = Time::new((2017, 6, 4), (9, 30, 15), crate::value::JST);
        let bytes = days(&t).to_le_bytes();
        let zone = zone(&header_zone());
        assert_eq!(time(&bytes, zone).unix(), t.unix());
    }

    #[test]
    fn zones() {
        assert_eq!(zone(&USE_UTC.to_le_bytes()), 0);
        assert_eq!(zone(&USE_JST.to_le_bytes()), 540);
        assert_eq!(zone(&(-540i16).to_le_bytes()), 540);
    }
}
