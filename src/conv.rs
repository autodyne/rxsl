use crate::model::{Item, Node, QName};
use crate::value::{Band, Time};

const BAND_RANGES: &[(u64, u64, u64)] = &[
    (135, 138, 135),
    (472, 479, 475),
    (1800, 1913, 1900),
    (3500, 3687, 3500),
    (7000, 7200, 7000),
    (10000, 10150, 10000),
    (14000, 14350, 14000),
    (18000, 18168, 18000),
    (21000, 21450, 21000),
    (24000, 24990, 24000),
    (28000, 29700, 28000),
    (50000, 54000, 50000),
    (144000, 146000, 144000),
    (430000, 440000, 430000),
    (1200000, 1300000, 1200000),
    (2400000, 2450000, 2400000),
    (5600000, 5850000, 5600000),
    (10000000, 10500000, 10000000),
];

const UPPER_BANDS: &[u64] = &[24000000, 47000000, 75000000, 77000000, 135000000, 248000000];

const ADIF_BANDS: &[(u64, &str)] = &[
    (135, "2190m"),
    (475, "630m"),
    (1900, "160m"),
    (3500, "80m"),
    (7000, "40m"),
    (10000, "30m"),
    (14000, "20m"),
    (18000, "17m"),
    (21000, "15m"),
    (24000, "12m"),
    (28000, "10m"),
    (50000, "6m"),
    (144000, "2m"),
    (430000, "70cm"),
    (1200000, "23cm"),
    (2400000, "13cm"),
    (5600000, "6cm"),
    (10000000, "3cm"),
    (24000000, "1.25cm"),
    (47000000, "6mm"),
    (77000000, "4mm"),
    (75000000, "4mm"),
    (135000000, "2mm"),
    (248000000, "1mm"),
];

const CQWW_BANDS: &[(u64, u64)] = &[
    (1900, 1800),
    (3500, 3500),
    (7000, 7000),
    (10000, 10000),
    (14000, 14000),
    (18000, 18000),
    (21000, 21000),
    (24000, 24000),
    (28000, 28000),
    (50000, 50000),
    (144000, 144000),
    (430000, 432000),
    (1200000, 1200000),
    (2400000, 2300000),
    (5600000, 5700000),
    (10000000, 10000000),
    (24000000, 24000000),
    (47000000, 47000000),
    (75000000, 75000000),
    (77000000, 75000000),
    (135000000, 134000000),
    (248000000, 241000000),
];

const CQWW_READ_BANDS: &[(u64, u64)] = &[
    (24500, 24000),
    (2300000, 2400000),
    (134000000, 135000000),
    (241000000, 248000000),
];

pub fn nominal_band(khz: &str) -> Option<String> {
    let hz = Band::parse(khz).ok()?.0;
    let in_band = |lo: u64, hi: u64| lo * 1000 <= hz && hz <= hi * 1000;
    BAND_RANGES
        .iter()
        .find(|(lo, hi, _)| in_band(*lo, *hi))
        .map(|b| b.2)
        .or_else(|| UPPER_BANDS.iter().copied().find(|b| b * 1000 == hz))
        .map(|b| b.to_string())
}

pub fn adif_band(name: &str) -> Option<String> {
    let name = name.to_lowercase();
    ADIF_BANDS
        .iter()
        .find(|(_, n)| *n == name)
        .map(|(k, _)| k.to_string())
}

pub fn adif_band_name(khz: &str) -> Option<&'static str> {
    let k = Band::parse(khz).ok()?;
    ADIF_BANDS
        .iter()
        .find(|(b, _)| Band::from_khz(*b) == k)
        .map(|(_, n)| *n)
}

pub fn cqww_read_band(khz: &str) -> Option<String> {
    let k = Band::parse(khz).ok()?;
    CQWW_READ_BANDS
        .iter()
        .find(|(c, _)| Band::from_khz(*c) == k)
        .map(|(_, b)| b.to_string())
}

pub fn cqww_band(khz: &str) -> Option<String> {
    let k = Band::parse(khz).ok()?;
    CQWW_BANDS
        .iter()
        .find(|(b, _)| Band::from_khz(*b) == k)
        .map(|(_, c)| c.to_string())
}

pub fn is_morse(mode: &str) -> bool {
    mode.eq_ignore_ascii_case("CW")
}

pub fn is_phone(mode: &str) -> bool {
    ["PH", "AM", "FM", "DSB", "SSB", "LSB", "USB"]
        .iter()
        .any(|m| mode.eq_ignore_ascii_case(m))
}

pub fn is_digit(mode: &str) -> bool {
    ["DG", "FT4", "FT8", "RTTY", "MFSK"]
        .iter()
        .any(|m| mode.eq_ignore_ascii_case(m))
}

pub fn cqww_mode(mode: &str) -> Option<&'static str> {
    match mode {
        "CW" => Some("CW"),
        "PH" => Some("SSB"),
        "FM" => Some("FM"),
        "RY" | "DG" => Some("RTTY"),
        "DV" => Some("DV"),
        _ => None,
    }
}

pub fn set_cqww_mode(mode: &str) -> Option<&'static str> {
    if is_morse(mode) {
        Some("CW")
    } else if mode.eq_ignore_ascii_case("FM") {
        Some("FM")
    } else if mode.eq_ignore_ascii_case("RTTY") {
        Some("RY")
    } else if is_phone(mode) {
        Some("PH")
    } else if is_digit(mode) {
        Some("DG")
    } else {
        None
    }
}

pub fn zlog_mode(mode: &str) -> String {
    if mode.eq_ignore_ascii_case("FT4") || mode.eq_ignore_ascii_case("FT8") {
        mode.to_ascii_uppercase()
    } else if is_digit(mode) {
        "RTTY".into()
    } else {
        mode.into()
    }
}

pub fn cut_rst_num(code: &str, phone: bool) -> (String, Option<String>) {
    if let Some((rst, num)) = code.split_once(' ') {
        return (rst.to_string(), Some(num.trim_start().to_string()));
    }
    let n = if phone { 2 } else { 3 };
    let chars: Vec<char> = code.chars().collect();
    if chars.len() <= n {
        return (code.to_string(), None);
    }
    (
        chars[..n].iter().collect(),
        Some(chars[n..].iter().collect()),
    )
}

pub fn join_code(node: &Node, phone: bool) -> Option<String> {
    let rst = node.qxsl("rstq");
    let code = node.qxsl("code");
    if rst.is_none() && code.is_none() {
        return None;
    }
    // A report of unusual length is set off by a space.
    let n = if phone { 2 } else { 3 };
    let gap = match (rst, code) {
        (Some(r), Some(_)) if r.chars().count() != n => " ",
        _ => "",
    };
    Some(format!("{}{gap}{}", rst.unwrap_or(""), code.unwrap_or("")))
}

fn q(local: &str) -> QName {
    QName::qxsl(local)
}

fn a(local: &str) -> QName {
    QName::adif(local)
}

fn rstq(value: Option<&str>) -> Option<String> {
    value.and_then(|v| v.trim().parse::<i32>().ok().map(|n| n.to_string()))
}

pub fn normalize(item: &Item, kind: &str) -> Item {
    let item = &peer_call_in_rcvd(item);
    let mut it = if kind == "jswl" || kind == "SWL" {
        normalize_jswl(item)
    } else if item.attrs.adif("APP_N1MM_EXCHANGE1").is_some() {
        normalize_n1mm(item)
    } else if item.attrs.adif("TIME_ON").is_some() {
        normalize_adif(item)
    } else if kind == "cqww" || is_cqww(item) {
        normalize_cqww(item)
    } else if item.rcvd.qxsl("rstq").is_some() {
        normalize_qxsl(item)
    } else {
        normalize_zdos(item)
    };
    if kind == "cqww" {
        if let Some(b) = it.attrs.qxsl("band").and_then(cqww_read_band) {
            it.attrs.set(q("band"), b);
        }
    }
    let band = it.attrs.qxsl("band").and_then(nominal_band);
    it.attrs.put(q("band"), band);
    it
}

pub fn transform(item: &Item, kind: &str) -> Item {
    let mut it = peer_call_in_rcvd(item);
    let exact = it.attrs.qxsl("band").map(str::to_string);
    let band = it.attrs.qxsl("band").and_then(nominal_band);
    it.attrs.put(q("band"), band);
    match kind {
        "adis" | "adxs" => transform_adif(&it, exact.as_deref()),
        "cqww" => transform_cqww(&it),
        "jarl" => {
            it.attrs.remove(&q("name"));
            it
        }
        "jswl" => {
            it.attrs.remove(&q("name"));
            it.sent.remove(&q("rstq"));
            it
        }
        "ctxt" => {
            let mut z = transform_zdos(&it);
            z.attrs.remove(&q("name"));
            z
        }
        "cbin" => transform_zdos(&it),
        "zall" | "zdos" | "zbin" | "znew" => {
            let mode = it.attrs.qxsl("mode").map(zlog_mode);
            it.attrs.put(q("mode"), mode);
            it
        }
        _ => it,
    }
}

fn is_cqww(item: &Item) -> bool {
    item.attrs
        .qxsl("mode")
        .is_some_and(|m| ["PH", "DG", "RY"].iter().any(|c| m.eq_ignore_ascii_case(c)))
}

fn normalize_qxsl(item: &Item) -> Item {
    let mut it = item.clone();
    for node in [&mut it.rcvd, &mut it.sent] {
        let r = rstq(node.qxsl("rstq"));
        node.put(q("rstq"), r);
    }
    it
}

fn peer_call_in_rcvd(item: &Item) -> Item {
    let mut it = item.clone();
    if let Some(call) = it.attrs.qxsl("call").map(str::to_string) {
        it.attrs.remove(&q("call"));
        if !it.rcvd.contains(&q("call")) {
            it.rcvd.set(q("call"), call);
        }
    }
    it
}

fn normalize_jswl(item: &Item) -> Item {
    let mut it = normalize_qxsl(item);
    let phone = it.attrs.qxsl("mode").is_some_and(is_phone);
    it.sent.set(q("rstq"), if phone { "59" } else { "599" });
    it
}

fn adif_time(attrs: &Node) -> Option<Time> {
    let d = attrs.adif("QSO_DATE")?;
    let t = attrs.adif("TIME_ON")?;
    if d.len() < 8 || t.len() < 4 || !d.is_ascii() || !t.is_ascii() {
        return None;
    }
    let mut iso = format!(
        "{}-{}-{}T{}:{}",
        &d[..4],
        &d[4..6],
        &d[6..8],
        &t[..2],
        &t[2..4]
    );
    if t.len() >= 6 {
        iso.push(':');
        iso.push_str(&t[4..6]);
    }
    iso.push('Z');
    Time::parse_iso(&iso).ok()
}

const ADIF_KEYS: &[&str] = &[
    "QSO_DATE",
    "TIME_ON",
    "CALL",
    "BAND",
    "MODE",
    "OPERATOR",
    "RST_RCVD",
    "RST_SENT",
    "SRX",
    "STX",
    "SRX_STRING",
    "STX_STRING",
    "STATION_CALLSIGN",
    "COMMENT",
];

fn normalize_adif(item: &Item) -> Item {
    let mut it = item.clone();
    let f = &item.attrs;
    let time = adif_time(f).map(|t| t.to_iso());
    for k in ADIF_KEYS {
        it.attrs.remove(&a(k));
    }
    it.attrs.put(q("time"), time);
    it.rcvd.put(q("call"), f.adif("CALL"));
    it.sent.put(q("call"), f.adif("STATION_CALLSIGN"));
    let freq = || {
        f.adif("FREQ")
            .and_then(|m| Band::parse(&format!("{}MHz", m.trim())).ok())
            .map(|b| b.khz())
    };
    let band = f.adif("BAND").and_then(adif_band).or_else(freq);
    it.attrs.put(q("band"), band);
    it.attrs.put(q("mode"), f.adif("MODE"));
    it.attrs.put(q("name"), f.adif("OPERATOR"));
    it.rcvd.put(q("rstq"), rstq(f.adif("RST_RCVD")));
    it.sent.put(q("rstq"), rstq(f.adif("RST_SENT")));
    it.rcvd
        .put(q("code"), f.adif("SRX").or(f.adif("SRX_STRING")));
    it.sent
        .put(q("code"), f.adif("STX").or(f.adif("STX_STRING")));
    it.attrs.put(q("note"), f.adif("COMMENT"));
    it
}

fn adif_code(code: Option<&str>) -> (Option<&str>, Option<&str>) {
    match code {
        Some(c) if !c.is_empty() && c.bytes().all(|b| b.is_ascii_digit()) => (Some(c), None),
        Some(c) => (None, Some(c)),
        None => (None, None),
    }
}

fn normalize_n1mm(item: &Item) -> Item {
    let mut it = normalize_adif(item);
    let code = item.attrs.adif("APP_N1MM_EXCHANGE1");
    it.attrs.remove(&a("APP_N1MM_EXCHANGE1"));
    it.rcvd.put(q("code"), code);
    it
}

fn normalize_cqww(item: &Item) -> Item {
    let mut it = normalize_qxsl(item);
    let mode = item.attrs.qxsl("mode").and_then(cqww_mode);
    it.attrs.put(q("mode"), mode);
    it
}

fn normalize_zdos(item: &Item) -> Item {
    let mut it = item.clone();
    let phone = item.attrs.qxsl("mode").is_some_and(is_phone);
    for node in [&mut it.rcvd, &mut it.sent] {
        if let Some(code) = node.qxsl("code").map(str::to_string) {
            let (rst, num) = cut_rst_num(&code, phone);
            node.put(q("rstq"), rstq(Some(&rst)));
            node.put(q("code"), num);
        }
    }
    it
}

fn transform_adif(item: &Item, exact: Option<&str>) -> Item {
    let mut it = item.clone();
    let f = &item.attrs;
    for k in ["time", "band", "mode", "name", "note"] {
        it.attrs.remove(&q(k));
    }
    for k in ["call", "rstq", "code"] {
        it.rcvd.remove(&q(k));
        it.sent.remove(&q(k));
    }
    if let Some(t) = f.qxsl("time").and_then(|t| Time::parse_iso(t).ok()) {
        let u = t.at_offset(0);
        let date = format!("{:04}{:02}{:02}", u.year, u.month, u.day);
        let time = format!("{:02}{:02}{:02}", u.hour, u.minute, u.second);
        it.attrs.set(a("QSO_DATE"), date);
        it.attrs.set(a("TIME_ON"), time);
    }
    it.attrs.put(a("CALL"), item.rcvd.qxsl("call"));
    it.attrs
        .put(a("BAND"), f.qxsl("band").and_then(adif_band_name));
    it.attrs.put(a("MODE"), f.qxsl("mode"));
    it.attrs.put(a("OPERATOR"), f.qxsl("name"));
    it.attrs.put(a("RST_RCVD"), item.rcvd.qxsl("rstq"));
    it.attrs.put(a("RST_SENT"), item.sent.qxsl("rstq"));
    let (srx, srx_string) = adif_code(item.rcvd.qxsl("code"));
    let (stx, stx_string) = adif_code(item.sent.qxsl("code"));
    it.attrs.put(a("SRX"), srx);
    it.attrs.put(a("STX"), stx);
    it.attrs.put(a("STATION_CALLSIGN"), item.sent.qxsl("call"));
    it.attrs.put(a("SRX_STRING"), srx_string);
    it.attrs.put(a("STX_STRING"), stx_string);
    if let Some(note) = f.qxsl("note") {
        it.attrs.set(a("COMMENT"), note);
    }
    let exact = exact.and_then(|e| Band::parse(e).ok());
    let nominal = f.qxsl("band").and_then(|b| Band::parse(b).ok());
    if let Some(e) = exact.filter(|e| Some(*e) != nominal) {
        if f.adif("FREQ").is_none() {
            it.attrs.set(a("FREQ"), e.mhz());
        }
    }
    it
}

fn transform_cqww(item: &Item) -> Item {
    let mut it = item.clone();
    it.attrs.remove(&q("name"));
    let band = item.attrs.qxsl("band").and_then(cqww_band);
    it.attrs.put(q("band"), band);
    let mode = item.attrs.qxsl("mode").and_then(set_cqww_mode);
    it.attrs.put(q("mode"), mode);
    it
}

fn transform_zdos(item: &Item) -> Item {
    let mut it = item.clone();
    let phone = item.attrs.qxsl("mode").is_some_and(is_phone);
    for node in [&mut it.rcvd, &mut it.sent] {
        let code = join_code(node, phone);
        node.remove(&q("rstq"));
        node.put(q("code"), code);
    }
    it
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables() {
        assert_eq!(nominal_band("14025").as_deref(), Some("14000"));
        assert_eq!(nominal_band("432000").as_deref(), Some("430000"));
        assert_eq!(nominal_band("2300000"), None);
        assert_eq!(adif_band("20M").as_deref(), Some("14000"));
        assert_eq!(adif_band_name("430000"), Some("70cm"));
        assert_eq!(cqww_band("1900").as_deref(), Some("1800"));
        assert_eq!(cqww_read_band("24500").as_deref(), Some("24000"));
        assert_eq!(set_cqww_mode("usb"), Some("PH"));
        assert_eq!(set_cqww_mode("FT8"), Some("DG"));
        assert_eq!(set_cqww_mode("FM"), Some("FM"));
        assert_eq!(set_cqww_mode("RTTY"), Some("RY"));
        assert_eq!(zlog_mode("FT8"), "FT8");
        assert_eq!(zlog_mode("ft4"), "FT4");
        assert_eq!(zlog_mode("MFSK"), "RTTY");
        assert_eq!(
            cut_rst_num("59913", true),
            ("59".into(), Some("913".into()))
        );
        assert_eq!(cut_rst_num("599", false), ("599".into(), None));
    }

    #[test]
    fn cabrillo_modes_are_read() {
        for (m, want) in [
            ("CW", "CW"),
            ("PH", "SSB"),
            ("FM", "FM"),
            ("RY", "RTTY"),
            ("DG", "RTTY"),
            ("DV", "DV"),
        ] {
            let mut it = Item::default();
            it.attrs.set(q("mode"), m);
            it.rcvd.set(q("rstq"), "599");
            let n = normalize(&it, "cqww");
            assert_eq!(n.attrs.qxsl("mode"), Some(want), "{m}");
        }
    }

    #[test]
    fn adif_round_trip_keeps_unknown_fields() {
        let mut it = Item::default();
        for (k, v) in [
            ("QSO_DATE", "20170604"),
            ("TIME_ON", "0915"),
            ("CALL", "JA1ZLO"),
            ("BAND", "20m"),
            ("MODE", "CW"),
            ("RST_RCVD", "599"),
            ("SRX", "13"),
            ("APP_N1MM_EXCHANGE1", "13M"),
            ("COMMENT", "hello"),
        ] {
            it.attrs.set(QName::adif(k), v);
        }
        let n = normalize(&it, "adis");
        assert_eq!(n.attrs.qxsl("time"), Some("2017-06-04T09:15:00Z"));
        assert_eq!(n.attrs.qxsl("band"), Some("14000"));
        assert_eq!(n.rcvd.qxsl("code"), Some("13M"));
        assert_eq!(n.rcvd.qxsl("call"), Some("JA1ZLO"));
        assert_eq!(n.sent.qxsl("call"), None);
        assert_eq!(n.attrs.qxsl("call"), None);
        assert_eq!(n.attrs.adif("COMMENT"), None);
        assert_eq!(n.attrs.qxsl("note"), Some("hello"));
        let t = transform(&n, "adis");
        assert_eq!(t.attrs.adif("TIME_ON"), Some("091500"));
        assert_eq!(t.attrs.adif("SRX"), None);
        assert_eq!(t.attrs.adif("SRX_STRING"), Some("13M"));
        assert_eq!(t.attrs.qxsl("note"), None);
        assert_eq!(t.attrs.adif("COMMENT"), Some("hello"));
        assert_eq!(t.attrs.qxsl("time"), None);
        assert_eq!(t.attrs.adif("CALL"), Some("JA1ZLO"));
        assert_eq!(t.attrs.adif("STATION_CALLSIGN"), None);
        assert_eq!(t.rcvd.qxsl("call"), None);
    }

    #[test]
    fn adif_station_callsign_is_the_sent_call() {
        let mut it = Item::default();
        it.attrs.set(QName::adif("TIME_ON"), "0915");
        it.attrs.set(QName::adif("CALL"), "QV1DOK");
        it.attrs.set(QName::adif("STATION_CALLSIGN"), "JA1ZLO");
        let n = normalize(&it, "adis");
        assert_eq!(n.rcvd.qxsl("call"), Some("QV1DOK"));
        assert_eq!(n.sent.qxsl("call"), Some("JA1ZLO"));
        assert_eq!(n.attrs.adif("STATION_CALLSIGN"), None);
        let t = transform(&n, "adis");
        assert_eq!(t.attrs.adif("CALL"), Some("QV1DOK"));
        assert_eq!(t.attrs.adif("STATION_CALLSIGN"), Some("JA1ZLO"));
        assert_eq!(t.sent.qxsl("call"), None);
    }

    #[test]
    fn item_level_call_moves_to_rcvd() {
        let mut it = Item::default();
        it.attrs.set(q("call"), "QV1DOK");
        it.rcvd.set(q("rstq"), "599");
        for out in [normalize(&it, "qxml"), transform(&it, "qxml")] {
            assert_eq!(out.attrs.qxsl("call"), None);
            assert_eq!(out.rcvd.qxsl("call"), Some("QV1DOK"));
        }
    }
}
