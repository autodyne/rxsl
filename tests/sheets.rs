use rxsl::formats::{cqww::Cqww, jarl::Jarl};
use rxsl::sheets::summary::{csv, parse_csv, Score, Summary, Version};
use rxsl::sheets::{cab3, cab3::Cab3Sheet, jarl::JarlSheet, sheet, sheets, unpack};
use rxsl::{Field, Format, Item, QName, Sheet, SheetFormat};

fn sample(name: &str) -> Vec<u8> {
    std::fs::read(format!("{}/tests/data/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

fn text(lines: &[&str]) -> String {
    lines.iter().map(|l| format!("{l}\r\n")).collect()
}

fn summary(version: Version) -> Summary {
    Summary {
        version,
        contest: "ALL JA1".into(),
        category_code: "PN".into(),
        category_name: "Newcomer".into(),
        callsign: "JA1ZLO".into(),
        op_callsign: "JA1ZLO".into(),
        scores: vec![
            Score {
                band: "7MHz".into(),
                qsos: 3,
                points: 2,
                mults: 1,
            },
            Score {
                band: "14MHz".into(),
                qsos: 1,
                points: 1,
                mults: 1,
            },
        ],
        total_score: "6".into(),
        address: "Tokyo\nBunkyo\n".into(),
        tel: "03-0000-0000".into(),
        name: "Taro".into(),
        email: "taro@example.org".into(),
        license_class: "1".into(),
        power: "10".into(),
        power_type: "\u{5b9a}\u{683c}\u{51fa}\u{529b}".into(),
        op_place: "Bunkyo".into(),
        power_supply: "AC".into(),
        equipment: "IC-7300".into(),
        license_date: Some("2025\u{5e74}03\u{6708}04\u{65e5}".into()),
        comments: "Thanks.".into(),
        club_number: "10001".into(),
        club_name: "Club".into(),
        oath: "I swear.".into(),
        date: "2026-04-26".into(),
        signature: "Taro".into(),
        ..Summary::default()
    }
}

const BODY: &str =
    "DATE (JST) TIME   BAND MODE  CALLSIGN      SENTNo      RCVDNo      Mlt    Pts\r\n";

#[test]
fn jarl_r1_byte_exact() {
    let mut sheet = summary(Version::R1).to_sheet();
    sheet.table_type = Some("JARL".into());
    sheet.table = BODY.as_bytes().to_vec();
    let bytes = JarlSheet.encode(&sheet).unwrap();
    let want = text(&[
        "<SUMMARYSHEET VERSION=R1.0>",
        "<CONTESTNAME>ALL JA1</CONTESTNAME>",
        "<CATEGORYCODE>PN</CATEGORYCODE>",
        "<CATEGORYNAME>Newcomer</CATEGORYNAME>",
        "<CALLSIGN>JA1ZLO</CALLSIGN>",
        "<OPCALLSIGN>JA1ZLO</OPCALLSIGN>",
        "<SCORE BAND=7MHz>3,2,1</SCORE>",
        "<SCORE BAND=14MHz>1,1,1</SCORE>",
        "<SCORE BAND=TOTAL>4,3,2</SCORE>",
        "<TOTALSCORE>6</TOTALSCORE>",
        "<ADDRESS>Tokyo",
        "Bunkyo",
        "</ADDRESS>",
        "<TEL>03-0000-0000</TEL>",
        "<NAME>Taro</NAME>",
        "<EMAIL>taro@example.org</EMAIL>",
        "<LICENSECLASS>1</LICENSECLASS>",
        "<POWER>10</POWER>",
        "<POWERTYPE>\u{5b9a}\u{683c}\u{51fa}\u{529b}</POWERTYPE>",
        "<OPPLACE>Bunkyo</OPPLACE>",
        "<POWERSUPPLY>AC</POWERSUPPLY>",
        "<EQUIPMENT>IC-7300",
        "</EQUIPMENT>",
        "<COMMENTS>Thanks.",
        "</COMMENTS>",
        "<REGCLUBNUMBER>10001</REGCLUBNUMBER>",
        "<REGCLUBNAME>Club</REGCLUBNAME>",
        "<OATH>I swear.",
        "</OATH>",
        "<DATE>2026-04-26</DATE>",
        "<SIGNATURE>Taro</SIGNATURE>",
        "</SUMMARYSHEET>",
        "<LOGSHEET TYPE=\"JARL\">",
    ]) + BODY
        + "</LOGSHEET>\r\n";
    let sjis = encoding_rs::SHIFT_JIS.encode(&want).0.into_owned();
    assert_eq!(bytes, sjis);
    let at = bytes.windows(11).position(|w| w == b"<POWERTYPE>").unwrap();
    assert_eq!(
        &bytes[at + 11..at + 19],
        &[0x92, 0xe8, 0x8a, 0x69, 0x8f, 0x6f, 0x97, 0xcd]
    );
}

#[test]
fn jarl_r2_byte_exact() {
    let mut s = summary(Version::R2);
    s.operators = vec!["JA1ZLO".into(), "JA1 YAA".into()];
    let bytes = JarlSheet.encode(&s.to_sheet()).unwrap();
    let want = text(&[
        "<SUMMARYSHEET VERSION=R2.1>",
        "<CONTESTNAME>ALL JA1</CONTESTNAME>",
        "<CATEGORYCODE>PN</CATEGORYCODE>",
        "<CALLSIGN>JA1ZLO</CALLSIGN>",
        "<OPCALLSIGN>JA1ZLO</OPCALLSIGN>",
        "<TOTALSCORE>6</TOTALSCORE>",
        "<ADDRESS>Tokyo",
        "Bunkyo",
        "</ADDRESS>",
        "<NAME>Taro</NAME>",
        "<TEL>03-0000-0000</TEL>",
        "<EMAIL>taro@example.org</EMAIL>",
        "<POWER>10</POWER>",
        "<OPPLACE>Bunkyo</OPPLACE>",
        "<POWERSUPPLY>AC</POWERSUPPLY>",
        "<LICENSEDATE>2025\u{5e74}03\u{6708}04\u{65e5}</LICENSEDATE>",
        "<COMMENTS>Thanks.",
        "</COMMENTS>",
        "<MULTIOPLIST>JA1ZLO,\"JA1 YAA\"</MULTIOPLIST>",
        "<REGCLUBNUMBER>10001</REGCLUBNUMBER>",
        "<OATH>I swear.",
        "</OATH>",
        "<DATE>2026-04-26</DATE>",
        "<SIGNATURE>Taro</SIGNATURE>",
        "</SUMMARYSHEET>",
        "<LOGSHEET>",
        "</LOGSHEET>",
    ]);
    assert_eq!(bytes, encoding_rs::SHIFT_JIS.encode(&want).0.into_owned());
}

#[test]
fn summary_round_trip() {
    for v in [Version::R1, Version::R2] {
        let mut s = summary(v);
        s.coeff = Some("2".into());
        s.age = Some("70".into());
        s.operators = vec!["JA1ZLO".into(), "JA1 \"Y\"".into()];
        let bytes = JarlSheet.encode(&s.to_sheet()).unwrap();
        let back = Summary::from_sheet(&JarlSheet.decode(&bytes).unwrap());
        assert_eq!(back.to_sheet(), s.to_sheet());
        assert_eq!(back.version, v);
        assert_eq!(back.address, "Tokyo\nBunkyo");
    }
}

#[test]
fn coefficient_and_unmappable_text() {
    let mut s = summary(Version::R1);
    s.coeff = Some("2".into());
    s.comments = "\u{1F600}".into();
    let t = String::from_utf8_lossy(&JarlSheet.encode(&s.to_sheet()).unwrap()).into_owned();
    assert!(t.contains("<SCORE BAND=TOTAL>4,3,2</SCORE>\r\n<FDCOEFF>2</FDCOEFF>\r\n<TOTALSCORE>"));
    assert!(t.contains("<COMMENTS>?\r\n</COMMENTS>"));
    let mut s = summary(Version::R2);
    s.coeff = Some("2".into());
    let t = String::from_utf8_lossy(&JarlSheet.encode(&s.to_sheet()).unwrap()).into_owned();
    assert!(t.contains("<POWER>10</POWER>\r\n<FDCOEFF>2</FDCOEFF>\r\n<OPPLACE>"));
}

#[test]
fn csv_lists() {
    let list = vec!["A".to_string(), "".into(), "B C".into(), "D\"E".into()];
    assert_eq!(csv(&list), "A,\"\",\"B C\",\"D\"\"E\"");
    assert_eq!(parse_csv(&csv(&list)), list);
    assert!(parse_csv("").is_empty());
}

fn items(n: usize) -> Vec<Item> {
    let bands = ["3500", "7000", "14000", "144000", "1200000", "5600000"];
    (0..n)
        .map(|i| {
            let mut it = Item::default();
            let q = QName::qxsl;
            it.attrs.set(
                q("time"),
                format!("2017-06-04T{:02}:{:02}:00+09:00", i / 60 % 24, i % 60),
            );
            it.attrs.set(q("band"), bands[i % bands.len()]);
            it.attrs.set(q("mode"), ["CW", "SSB", "FM"][i % 3]);
            it.rcvd.set(q("call"), format!("JA{}ABC", i % 10));
            it.rcvd.set(q("rstq"), "599");
            it.rcvd.set(q("code"), format!("{:06}", 100000 + i));
            it.sent.set(q("rstq"), "599");
            it.sent.set(q("code"), "100110");
            it
        })
        .collect()
}

#[test]
fn jarl_sheet_embeds_the_log() {
    for n in [0, 1, 7, 100] {
        let list = items(n);
        let mut s = Sheet::default();
        s.set("CALLSIGN", "JA1ZLO");
        s.set("COMMENTS", "Groovy");
        s.embed(&Jarl::default(), &list).unwrap();
        let bytes = JarlSheet.encode(&s).unwrap();
        let back = JarlSheet.decode(&bytes).unwrap();
        assert_eq!(back.get("CALLSIGN"), Some("JA1ZLO"));
        assert_eq!(back.get("comments"), Some("Groovy"));
        assert_eq!(back.table, s.table);
        let want = Jarl::default().read(&s.table).unwrap();
        assert_eq!(back.items(&Jarl::default()).unwrap(), want);
        if n > 0 {
            assert_eq!(unpack(&bytes).unwrap(), want);
        }
    }
}

#[test]
fn cabrillo_sheet_embeds_the_log() {
    for n in [1, 7, 100] {
        let list = items(n);
        let mut s = Sheet::default();
        s.set("CONTEST", "JIDX-CW");
        s.set("CALLSIGN", "JA1ZLO");
        s.embed(&Cqww, &list).unwrap();
        let bytes = Cab3Sheet::default().encode(&s).unwrap();
        let back = Cab3Sheet::default().decode(&bytes).unwrap();
        assert_eq!(back.version, "3.0");
        assert_eq!(back.get("CONTEST"), Some("JIDX-CW"));
        assert_eq!(back.get("CALLSIGN"), Some("JA1ZLO"));
        let want = Cqww.read(&s.table).unwrap();
        assert_eq!(back.items(&Cqww).unwrap(), want);
        assert_eq!(unpack(&bytes).unwrap(), want);
    }
}

#[test]
fn cabrillo_header_byte_exact() {
    let fields = cab3::header(&[
        ("CALLSIGN", "JA1ZLO/1"),
        ("CONTEST", "JA-ALL-JA1"),
        ("CATEGORY-OPERATOR", "SINGLE-OP"),
        ("CLAIMED-SCORE", "6"),
        ("CREATED-BY", "RXSL"),
        ("X-NOTE", "extra"),
    ]);
    let mut s = Sheet {
        fields,
        ..Sheet::default()
    };
    s.push(Field::new("ADDRESS", "Tokyo\nBunkyo"));
    let qso = "QSO:  7003 CW 2026-04-25 1203 JA1ZLO/1      599 10H    JA1ZLO        599 13M    0";
    s.table = format!("START-OF-LOG: 3.0\r\nCALLSIGN: X\r\n{qso}\r\nX-{qso}\r\nEND-OF-LOG:\r\n")
        .into_bytes();
    let bytes = Cab3Sheet::default().encode(&s).unwrap();
    let mut want = vec!["START-OF-LOG: 3.0"];
    let tags: Vec<String> = cab3::TAGS
        .iter()
        .map(|t| {
            let v = match *t {
                "CALLSIGN" => "JA1ZLO/1",
                "CONTEST" => "JA-ALL-JA1",
                "CATEGORY-OPERATOR" => "SINGLE-OP",
                "CLAIMED-SCORE" => "6",
                "CREATED-BY" => "RXSL",
                _ => "",
            };
            format!("{t}: {v}")
        })
        .collect();
    want.extend(tags.iter().map(String::as_str));
    let x = format!("X-{qso}");
    want.extend([
        "X-NOTE: extra",
        "ADDRESS: Tokyo",
        "ADDRESS: Bunkyo",
        qso,
        &x,
        "END-OF-LOG:",
    ]);
    assert_eq!(String::from_utf8(bytes.clone()).unwrap(), text(&want));
    let back = Cab3Sheet::default().decode(&bytes).unwrap();
    assert_eq!(back.get("CATEGORY-ASSISTED"), Some(""));
    assert_eq!(back.all("ADDRESS").count(), 3);
    assert_eq!(Cqww.read(&back.table).unwrap().len(), 2);
}

#[test]
fn sample_log_in_a_sheet() {
    let mut s = Summary::default().to_sheet();
    s.table = sample("allja1.jarl");
    let bytes = JarlSheet.encode(&s).unwrap();
    assert_eq!(
        unpack(&bytes).unwrap(),
        Jarl::default().read(&s.table).unwrap()
    );
    let raw = sample("allja1.cqww");
    assert_eq!(unpack(&raw).unwrap(), Cqww.read(&raw).unwrap());
}

#[test]
fn registry() {
    let kinds: Vec<_> = sheets().iter().map(|s| s.kind()).collect();
    assert_eq!(kinds, ["jarl", "cab3"]);
    assert_eq!(sheet("cab3").unwrap().kind(), "cab3");
    assert_eq!(sheet("JARL R2.1").unwrap().kind(), "jarl");
    assert_eq!(sheet("CBR").unwrap().kind(), "cab3");
    assert!(sheet("adi").is_none());
    assert!(JarlSheet.decode(b"QSO: x").is_err());
    assert!(Cab3Sheet::default().decode(b"<SUMMARYSHEET>").is_err());
    assert!(JarlSheet
        .decode(b"<SUMMARYSHEET><CALLSIGN>X</SUMMARYSHEET>")
        .is_err());
}

#[test]
fn quoted_and_bare_attributes_are_read() {
    let src = "<SUMMARYSHEET VERSION=R2.1>\r\n<CALLSIGN>JA1ZLO</CALLSIGN>\r\n</SUMMARYSHEET>\r\n<LOGSHEET TYPE=\"CTESTWIN\">\r\nBODY\r\n</LOGSHEET>\r\n";
    let s = JarlSheet.decode(src.as_bytes()).unwrap();
    assert_eq!(s.version, "R2.1");
    assert_eq!(s.table_type.as_deref(), Some("CTESTWIN"));
    assert_eq!(s.table, b"BODY\r\n");
    assert_eq!(JarlSheet.encode(&s).unwrap(), src.as_bytes());
}

#[test]
fn cabrillo_in_utf8() {
    let mut s = Sheet::default();
    s.set("NAME", "Taro \u{592a}\u{90ce}");
    s.table = b"QSO: 7000 CW 2017-06-04 0015 JA1ZLO 599 1 QV1DOK 599 2 0\r\n".to_vec();
    let ascii = Cab3Sheet::default().encode(&s).unwrap();
    assert!(String::from_utf8(ascii.clone())
        .unwrap()
        .contains("NAME: Taro ??\r\n"));
    let utf8 = Cab3Sheet { utf8: true }.encode(&s).unwrap();
    let back = Cab3Sheet::default().decode(&utf8).unwrap();
    assert_eq!(back.get("NAME"), Some("Taro \u{592a}\u{90ce}"));
    assert_eq!(back.table, s.table);
    let mut latin = ascii.clone();
    latin.push(0xff);
    assert_eq!(
        Cab3Sheet::default().decode(&latin).unwrap().get("NAME"),
        Some("Taro ??")
    );
}

#[test]
fn category_rules() {
    use rxsl::sheets::summary::{
        date_text, license_date_text, oath, power_type, Missing, Rules, ADDRESS, OATH_JARL,
    };
    let rules = Rules::default();
    let mut s = summary(Version::R2);
    s.set_category(&rules, None, "70");
    assert_eq!(
        s.license_date.as_deref(),
        Some("2000\u{5e74}01\u{6708}01\u{65e5}")
    );
    assert_eq!(s.age, None);
    assert_eq!(s.missing(&rules), [Missing::LicenseDate]);
    s.set_category(&rules, Some((2025, 3, 4)), "");
    assert_eq!(s.license_date, Some(license_date_text((2025, 3, 4))));
    assert!(s.missing(&rules).is_empty());
    s.category_code = "XS".into();
    s.signature.clear();
    s.set_category(&rules, None, "");
    assert_eq!(s.license_date, None);
    assert_eq!(s.missing(&rules), [Missing::Signature, Missing::Age]);
    s.set_category(&rules, None, "70");
    assert_eq!(s.age.as_deref(), Some("70"));
    s.version = Version::R1;
    s.set_category(&rules, None, "70");
    assert_eq!((s.license_date.clone(), s.age.clone()), (None, None));
    s.category_code = "PN".into();
    assert_eq!(s.missing(&rules), [Missing::Signature]);
    assert_eq!(date_text((2026, 4, 5)), "2026\u{5e74}4\u{6708}5\u{65e5}");
    assert_eq!(oath(true), OATH_JARL);
    assert!(oath(false).contains("\u{4e3b}\u{50ac}\u{8005}"));
    assert_eq!(power_type(false), "\u{5b9a}\u{683c}\u{51fa}\u{529b}");
    assert_eq!(ADDRESS, "\u{3012}");
    s.set_operators(["JA1AAA", "", " JA1BBB ", "JA1AAA"]);
    assert_eq!(s.operators, ["JA1AAA", "JA1BBB"]);
}
