use std::collections::HashSet;

use crate::local::{CityBase, CityItem};
use crate::model::{Item, QName};
use crate::value::{local_offset, Date, Time};
use crate::Band;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Element {
    Null,
    Value(String),
    List(Vec<Element>),
}

impl Element {
    pub fn list(values: impl IntoIterator<Item = Element>) -> Self {
        Element::List(values.into_iter().collect())
    }

    pub fn size(&self) -> usize {
        match self {
            Element::List(v) => v.len(),
            _ => 1,
        }
    }

    pub fn get(&self, index: usize) -> Element {
        match self {
            Element::List(v) => v.get(index).cloned().unwrap_or(Element::Null),
            e if index == 0 => e.clone(),
            _ => Element::Null,
        }
    }
}

impl From<&str> for Element {
    fn from(value: &str) -> Self {
        Element::Value(value.into())
    }
}

impl From<String> for Element {
    fn from(value: String) -> Self {
        Element::Value(value)
    }
}

impl<T: Into<Element>> From<Option<T>> for Element {
    fn from(value: Option<T>) -> Self {
        value.map_or(Element::Null, Into::into)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    Success { item: Item, score: i32 },
    Failure { item: Item, text: String },
}

impl Message {
    pub fn success(item: Item, score: i32) -> Self {
        Message::Success { item, score }
    }

    pub fn failure(item: Item, text: impl ToString) -> Self {
        let text = text.to_string();
        Message::Failure { item, text }
    }

    pub fn item(&self) -> &Item {
        match self {
            Message::Success { item, .. } | Message::Failure { item, .. } => item,
        }
    }

    pub fn score(&self) -> i32 {
        match self {
            Message::Success { score, .. } => *score,
            Message::Failure { .. } => 0,
        }
    }

    pub fn text(&self) -> &str {
        match self {
            Message::Success { .. } => "",
            Message::Failure { text, .. } => text,
        }
    }

    pub fn is_success(&self) -> bool {
        matches!(self, Message::Success { .. })
    }

    pub fn is_failure(&self) -> bool {
        matches!(self, Message::Failure { .. })
    }
}

pub const DUPE: &str = "DUPE";

#[derive(Debug, Clone)]
pub struct Summary {
    accepted: Vec<Message>,
    rejected: Vec<Message>,
    entities: Vec<Element>,
    total: i32,
}

impl Summary {
    pub fn new<S: Section + ?Sized>(section: &S, list: &[Item]) -> Self {
        let mut keys = HashSet::new();
        let mut summary = Summary {
            accepted: Vec::new(),
            rejected: Vec::new(),
            entities: Vec::new(),
            total: 0,
        };
        for item in list {
            let msg = section.verify(item);
            if msg.is_failure() {
                summary.rejected.push(msg);
            } else if keys.insert(section.unique(msg.item())) {
                summary.accepted.push(msg);
            } else {
                let item = msg.item().clone();
                summary.rejected.push(Message::failure(item, DUPE));
            }
        }
        let entities = summary.accepted.iter().map(|m| section.entity(m.item()));
        summary.entities = entities.collect();
        if !summary.accepted.is_empty() {
            summary.total = section.result(&summary);
        }
        summary
    }

    pub fn accepted(&self) -> &[Message] {
        &self.accepted
    }

    pub fn rejected(&self) -> &[Message] {
        &self.rejected
    }

    pub fn score(&self) -> i32 {
        self.accepted.iter().map(Message::score).sum()
    }

    pub fn total(&self) -> i32 {
        self.total
    }

    pub fn keys(&self, rank: usize) -> HashSet<Element> {
        self.entities.iter().map(|e| e.get(rank)).collect()
    }

    pub fn key_sets(&self) -> Vec<HashSet<Element>> {
        let ranks = self.entities.iter().map(Element::size).min().unwrap_or(0);
        (0..ranks).map(|r| self.keys(r)).collect()
    }
}

pub trait Section: Send + Sync {
    fn name(&self) -> &str;

    fn code(&self) -> &str;

    fn is_absence(&self) -> bool {
        false
    }

    fn verify(&self, item: &Item) -> Message;

    fn unique(&self, item: &Item) -> Element;

    fn entity(&self, item: &Item) -> Element;

    fn result(&self, summary: &Summary) -> i32;

    fn summarize(&self, list: &[Item]) -> Summary {
        Summary::new(self, list)
    }

    fn award_limit(&self, scores: &[i32]) -> usize {
        ((0.1 * scores.len() as f64).ceil() as usize).min(7)
    }

    fn city_list(&self) -> Vec<CityItem> {
        Vec::new()
    }

    fn city_base(&self) -> CityBase {
        CityBase::new(self.city_list())
    }
}

pub struct Absence {
    pub name: String,
    pub code: String,
}

impl Absence {
    pub fn new(name: impl Into<String>, code: impl Into<String>) -> Self {
        let (name, code) = (name.into(), code.into());
        Absence { name, code }
    }
}

impl Section for Absence {
    fn name(&self) -> &str {
        &self.name
    }

    fn code(&self) -> &str {
        &self.code
    }

    fn is_absence(&self) -> bool {
        true
    }

    fn verify(&self, item: &Item) -> Message {
        Message::failure(item.clone(), "N/A")
    }

    fn unique(&self, _: &Item) -> Element {
        Element::list([])
    }

    fn entity(&self, _: &Item) -> Element {
        Element::list([])
    }

    fn result(&self, _: &Summary) -> i32 {
        0
    }
}

pub struct DefaultSection {
    pub name: String,
    pub code: String,
    pub bands: Option<Vec<Band>>,
    pub modes: Option<Vec<String>>,
    pub hours: Option<Vec<u32>>,
    pub offset: i32,
    pub codes: Option<CityBase>,
}

impl DefaultSection {
    pub fn new(name: impl Into<String>, code: impl Into<String>) -> Self {
        DefaultSection {
            name: name.into(),
            code: code.into(),
            bands: None,
            modes: None,
            hours: None,
            offset: local_offset(),
            codes: None,
        }
    }

    pub fn city_of(&self, code: &str) -> String {
        let stripped = code.strip_prefix("599").or_else(|| code.strip_prefix("59"));
        stripped.unwrap_or(code).to_string()
    }

    pub fn points(&self, _: &Item) -> i32 {
        1
    }
}

fn qxsl<'a>(node: &'a crate::Node, local: &str) -> &'a str {
    node.get(&QName::qxsl(local)).unwrap_or_default()
}

impl Section for DefaultSection {
    fn name(&self) -> &str {
        &self.name
    }

    fn code(&self) -> &str {
        &self.code
    }

    fn verify(&self, item: &Item) -> Message {
        let band = qxsl(&item.attrs, "band");
        let mode = qxsl(&item.attrs, "mode");
        let code = qxsl(&item.rcvd, "code");
        let time = qxsl(&item.attrs, "time");
        let hz = Band::parse(band).ok();
        if let Some(bands) = &self.bands {
            if !hz.is_some_and(|b| bands.contains(&b)) {
                return Message::failure(item.clone(), band);
            }
        }
        if let Some(modes) = &self.modes {
            if !modes.iter().any(|m| m.eq_ignore_ascii_case(mode)) {
                return Message::failure(item.clone(), mode);
            }
        }
        if let Some(hours) = &self.hours {
            let hour = Time::parse_iso(time).map(|t| t.at_offset(self.offset).hour);
            if !hour.is_ok_and(|h| hours.contains(&h)) {
                return Message::failure(item.clone(), time);
            }
        }
        if let Some(codes) = &self.codes {
            if !codes.contains_code(&self.city_of(code)) {
                return Message::failure(item.clone(), code);
            }
        }
        Message::success(item.clone(), self.points(item))
    }

    fn unique(&self, item: &Item) -> Element {
        let call = item.rcvd.get(&QName::qxsl("call"));
        let band = qxsl(&item.attrs, "band");
        Element::list([call.into(), band.into()])
    }

    fn entity(&self, item: &Item) -> Element {
        let band = qxsl(&item.attrs, "band");
        let city = self.city_of(qxsl(&item.rcvd, "code"));
        Element::list([Element::list([band.into(), city.into()])])
    }

    fn result(&self, summary: &Summary) -> i32 {
        let score = summary.score();
        if score > 0 {
            score * summary.keys(0).len() as i32
        } else {
            0
        }
    }

    fn city_list(&self) -> Vec<CityItem> {
        self.codes
            .as_ref()
            .map(|c| c.to_list().to_vec())
            .unwrap_or_default()
    }
}

pub trait Contest: Send + Sync {
    fn name(&self) -> &str;

    fn sections(&self) -> &[Box<dyn Section>];

    fn section(&self, name: &str) -> Option<&dyn Section> {
        let mut all = self.sections().iter();
        all.find(|s| s.name() == name).map(|s| s.as_ref())
    }

    fn similar(&self, name: &str) -> Option<&dyn Section> {
        let names = self.sections().iter().map(|s| s.name());
        self.section(crate::local::similar(name, names)?)
    }

    fn section_names(&self) -> Vec<&str> {
        unique(self.sections().iter().map(|s| s.name()))
    }

    fn section_codes(&self) -> Vec<&str> {
        unique(self.sections().iter().map(|s| s.code()))
    }
}

fn unique<'a>(all: impl Iterator<Item = &'a str>) -> Vec<&'a str> {
    let mut out: Vec<&str> = Vec::new();
    for s in all {
        if !out.contains(&s) {
            out.push(s);
        }
    }
    out
}

pub trait Program: Contest {
    fn host(&self) -> &str;

    fn mail(&self) -> &str;

    fn link(&self) -> &str;

    fn help(&self) -> &str;

    fn start_day(&self, year: i32) -> Date;

    fn final_day(&self, year: i32) -> Date;

    fn dead_line(&self, year: i32) -> Date;

    fn year(&self) -> i32 {
        let year = Date::today().year;
        let past = self.start_day(year).months_until(&Date::today()) % 12;
        if past >= 6 {
            year + 1
        } else {
            year
        }
    }

    fn accept(&self) -> bool {
        self.accept_at(local_offset())
    }

    fn finish(&self) -> bool {
        self.finish_at(local_offset())
    }

    fn accept_at(&self, offset: i32) -> bool {
        today(offset) <= self.dead_line(self.year())
    }

    fn finish_at(&self, offset: i32) -> bool {
        self.dead_line(self.year()) < today(offset)
    }

    fn limit_multiple_entry(&self, code: &str) -> usize {
        let _ = code;
        1
    }

    fn conflict(&self, entries: &[&dyn Section]) -> bool {
        entries.len() > 1
    }

    fn explain(&self, entries: &[&dyn Section]) -> Vec<String> {
        let _ = entries;
        Vec::new()
    }
}

fn today(offset: i32) -> Date {
    let t = Time::now(offset);
    Date::new(t.year, t.month, t.day)
}

pub trait Pattern: Send + Sync {
    fn matches(&self, a: &Item, b: &Item) -> bool {
        crate::conv::cross_check(a, b)
    }

    fn normalize(&self, item: &Item, kind: &str) -> Item {
        crate::conv::normalize(item, kind)
    }

    fn transform(&self, item: &Item, kind: &str) -> Item {
        crate::conv::transform(item, kind)
    }

    fn normalize_all(&self, list: &[Item], kind: &str) -> Vec<Item> {
        list.iter().map(|i| self.normalize(i, kind)).collect()
    }

    fn transform_all(&self, list: &[Item], kind: &str) -> Vec<Item> {
        list.iter().map(|i| self.transform(i, kind)).collect()
    }
}

pub struct DefaultPattern;

impl Pattern for DefaultPattern {}
