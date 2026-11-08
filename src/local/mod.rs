use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CityItem {
    path: Vec<String>,
}

impl CityItem {
    pub fn new(path: Vec<String>) -> Self {
        CityItem { path }
    }

    fn parse(line: &str) -> Self {
        let mut path: Vec<String> = Vec::new();
        for word in line.split_whitespace() {
            if !path.iter().any(|p| p == word) {
                path.push(word.into());
            }
        }
        CityItem { path }
    }

    pub fn path(&self) -> &[String] {
        &self.path
    }

    pub fn code(&self) -> &str {
        self.path.last().map(String::as_str).unwrap_or_default()
    }

    pub fn area(&self) -> String {
        self.name_at(0).unwrap_or_default()
    }

    pub fn name(&self) -> String {
        self.name_at(self.path.len().saturating_sub(2))
            .unwrap_or_default()
    }

    pub fn name_at(&self, level: usize) -> Option<String> {
        (level + 1 < self.path.len()).then(|| self.path[..=level].join(" "))
    }
}

impl std::fmt::Display for CityItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name())
    }
}

#[derive(Debug, Clone, Default)]
pub struct CityBase {
    list: Vec<CityItem>,
    codes: HashMap<String, usize>,
    names: HashMap<String, usize>,
}

impl CityBase {
    pub fn new(list: Vec<CityItem>) -> Self {
        let codes = list.iter().enumerate();
        let codes = codes.map(|(i, c)| (c.code().to_string(), i)).collect();
        let names = list.iter().enumerate();
        let names = names.map(|(i, c)| (c.name(), i)).collect();
        CityBase { list, codes, names }
    }

    pub fn read(text: &str) -> Self {
        let lines = text.lines().filter(|l| !l.trim().is_empty());
        CityBase::new(lines.map(CityItem::parse).collect())
    }

    pub fn load(name: &str) -> Option<&'static CityBase> {
        match name.rsplit('/').next()? {
            "city.ja" => Some(cities()),
            "area.ja" => Some(areas()),
            _ => None,
        }
    }

    pub fn to_list(&self) -> &[CityItem] {
        &self.list
    }

    pub fn contains_code(&self, code: &str) -> bool {
        self.codes.contains_key(code)
    }

    pub fn by_code(&self, code: &str) -> Option<&CityItem> {
        self.codes.get(code).map(|i| &self.list[*i])
    }

    pub fn by_name(&self, name: &str) -> Option<&CityItem> {
        self.names.get(name).map(|i| &self.list[*i])
    }

    pub fn recommend(&self, name: &str) -> Option<&CityItem> {
        let names = self.list.iter().map(CityItem::name);
        let best = similar(name, names)?;
        self.by_name(&best)
    }
}

pub fn cities() -> &'static CityBase {
    static BASE: OnceLock<CityBase> = OnceLock::new();
    BASE.get_or_init(|| CityBase::read(include_str!("city.ja")))
}

pub fn areas() -> &'static CityBase {
    static BASE: OnceLock<CityBase> = OnceLock::new();
    BASE.get_or_init(|| CityBase::read(include_str!("area.ja")))
}

pub(crate) fn distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut next = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            let rep = prev[j] + (ca != cb) as usize;
            next[j + 1] = rep.min(prev[j + 1] + 1).min(next[j] + 1);
        }
        prev = next;
    }
    prev[b.len()]
}

pub(crate) fn similar<S: AsRef<str>>(anchor: &str, list: impl IntoIterator<Item = S>) -> Option<S> {
    list.into_iter()
        .map(|s| (distance(anchor, s.as_ref()), s))
        .min_by_key(|(d, _)| *d)
        .map(|(_, s)| s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables() {
        assert_eq!(cities().to_list().len(), 1407);
        assert_eq!(areas().to_list().len(), 47);
        assert!(CityBase::load("qxsl/local/city.ja").is_some());
    }

    #[test]
    fn items() {
        let item = cities().by_code("101").unwrap();
        assert_eq!(item.name(), "北海道 宗谷");
        assert_eq!(item.area(), "北海道");
        assert_eq!(cities().by_code("01").unwrap().name(), "北海道");
        assert_eq!(
            cities().by_name("東京都 千代田区").unwrap().code(),
            "100101"
        );
        let area = areas().by_code("10").unwrap();
        assert_eq!(area.area(), "1");
    }

    #[test]
    fn recommend() {
        let item = cities().recommend("東京都 千代田").unwrap();
        assert_eq!(item.code(), "100101");
    }

    #[test]
    fn distances() {
        assert_eq!(distance("kitten", "sitting"), 3);
        assert_eq!(distance("", "abc"), 3);
        assert_eq!(distance("東京", "東京都"), 1);
    }

    #[test]
    fn nearest() {
        assert_eq!(similar("ALLJA", ["UEC", "ALLJA1", "RTC"]), Some("ALLJA1"));
    }
}
