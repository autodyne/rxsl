use rxsl::local::cities;
use rxsl::ruler::{Absence, Contest, DefaultSection, Element, Message, Program, Section};
use rxsl::value::JST;
use rxsl::{Band, Date, Format, Item, QName, Weekday};

fn sample() -> Vec<Item> {
    let path = format!("{}/tests/data/allja1.qxml", env!("CARGO_MANIFEST_DIR"));
    rxsl::table::qxml::Qxml
        .read(&std::fs::read(path).unwrap())
        .unwrap()
}

fn section() -> DefaultSection {
    let mut s = DefaultSection::new("14MHz CW", "HB");
    s.bands = Some(vec![Band::from_khz(14000)]);
    s.modes = Some(vec!["CW".into()]);
    s.hours = Some(vec![9, 10, 11]);
    s.offset = JST;
    s.codes = Some(rxsl::local::CityBase::new(cities().to_list().to_vec()));
    s
}

#[test]
fn default_section_summarizes() {
    let s = section();
    let items = sample();
    let summary = s.summarize(&items);
    assert_eq!(
        summary.accepted().len() + summary.rejected().len(),
        items.len()
    );
    assert!(summary.accepted().iter().all(Message::is_success));
    assert!(summary.rejected().iter().any(|m| m.text() == "DUPE"));
    let mults = summary.keys(0).len() as i32;
    assert_eq!(summary.total(), summary.score() * mults);
    assert_eq!(summary.key_sets().len(), 1);
}

#[test]
fn absence_rejects_everything() {
    let a = Absence::new("checklog", "HB");
    let summary = a.summarize(&sample());
    assert!(a.is_absence());
    assert_eq!(summary.total(), 0);
    assert!(summary.rejected().iter().all(|m| m.text() == "N/A"));
}

#[test]
fn elements() {
    let e = Element::list([Element::list(["7000".into(), "10".into()]), Element::Null]);
    assert_eq!(e.size(), 2);
    assert_eq!(e.get(1), Element::Null);
    assert_eq!(e.get(0).get(1), Element::from("10"));
}

struct Test {
    sections: Vec<Box<dyn Section>>,
}

impl Contest for Test {
    fn name(&self) -> &str {
        "TEST"
    }

    fn sections(&self) -> &[Box<dyn Section>] {
        &self.sections
    }
}

impl Program for Test {
    fn host(&self) -> &str {
        "host"
    }

    fn mail(&self) -> &str {
        "mail"
    }

    fn link(&self) -> &str {
        "link"
    }

    fn help(&self) -> &str {
        ""
    }

    fn start_day(&self, year: i32) -> Date {
        Date::nth_weekday(year, 6, 4, Weekday::Saturday)
    }

    fn final_day(&self, year: i32) -> Date {
        self.start_day(year)
    }

    fn dead_line(&self, year: i32) -> Date {
        self.start_day(year).plus_days(21)
    }
}

#[test]
fn programs() {
    let t = Test {
        sections: vec![
            Box::new(section()),
            Box::new(Absence::new("checklog", "HB")),
        ],
    };
    assert_eq!(t.section_names(), ["14MHz CW", "checklog"]);
    assert_eq!(t.section_codes(), ["HB"]);
    assert_eq!(t.similar("14MHz").unwrap().name(), "14MHz CW");
    assert!(t.section("none").is_none());
    assert_eq!(t.accept(), !t.finish());
    let a = t.sections()[0].as_ref();
    assert!(t.conflict(&[a, a]));
    assert!(!t.conflict(&[a]));
    assert_eq!(t.limit_multiple_entry("HB"), 1);
    assert_eq!(a.award_limit(&[1; 100]), 7);
    assert_eq!(a.award_limit(&[1; 15]), 2);
}

#[test]
fn cross_check_pairs_contacts() {
    let mut a = Item::default();
    a.attrs.set(QName::qxsl("time"), "2017-06-04T00:00:00Z");
    a.attrs.set(QName::qxsl("band"), "7000");
    a.attrs.set(QName::qxsl("mode"), "CW");
    a.sent.set(QName::qxsl("code"), "100110");
    a.rcvd.set(QName::qxsl("code"), "1002");
    let mut b = a.clone();
    b.attrs.set(QName::qxsl("time"), "2017-06-04T00:02:00Z");
    b.sent.set(QName::qxsl("code"), "1002");
    b.rcvd.set(QName::qxsl("code"), "100110");
    assert!(rxsl::conv::cross_check(&a, &b));
    b.attrs.set(QName::qxsl("time"), "2017-06-04T00:04:00Z");
    assert!(!rxsl::conv::cross_check(&a, &b));
}

#[test]
fn table_manager() {
    assert_eq!(rxsl::format("zbin").unwrap().kind(), "zbin");
    assert_eq!(rxsl::format("zLog (ZLOX)").unwrap().kind(), "znew");
    assert_eq!(rxsl::format("ZLO").unwrap().kind(), "zbin");
    assert!(rxsl::format("none").is_none());
    let items = sample();
    let bytes = rxsl::encode(&items).unwrap();
    assert_eq!(rxsl::decode(&bytes).unwrap().len(), items.len());
    assert!(rxsl::decode(b"\x00\x01garbage").is_err());
}
