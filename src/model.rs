pub const QXSL: &str = "qxsl.org";

pub const ADIF: &str = "adif.org";

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct QName {
    pub space: String,
    pub local: String,
}

impl QName {
    pub fn new(space: impl Into<String>, local: impl Into<String>) -> Self {
        QName {
            space: space.into(),
            local: local.into(),
        }
    }

    pub fn qxsl(local: &str) -> Self {
        QName::new(QXSL, local)
    }

    pub fn adif(local: &str) -> Self {
        QName::new(ADIF, local)
    }

    fn is(&self, space: &str, local: &str) -> bool {
        self.space == space && self.local == local
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Node {
    attrs: Vec<(QName, String)>,
}

impl Node {
    pub fn get_in(&self, space: &str, local: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k.is(space, local))
            .map(|(_, v)| v.as_str())
    }

    pub fn get(&self, key: &QName) -> Option<&str> {
        self.get_in(&key.space, &key.local)
    }

    pub fn qxsl(&self, local: &str) -> Option<&str> {
        self.get_in(QXSL, local)
    }

    pub fn adif(&self, local: &str) -> Option<&str> {
        self.get_in(ADIF, local)
    }

    pub fn set(&mut self, key: QName, value: impl Into<String>) {
        let value = value.into();
        match self.attrs.iter_mut().find(|(k, _)| *k == key) {
            Some(slot) => slot.1 = value,
            None => self.attrs.push((key, value)),
        }
    }

    pub fn put(&mut self, key: QName, value: Option<impl Into<String>>) {
        match value {
            Some(v) => self.set(key, v),
            None => self.remove(&key),
        }
    }

    pub fn remove(&mut self, key: &QName) {
        self.attrs.retain(|(k, _)| k != key);
    }

    pub fn contains(&self, key: &QName) -> bool {
        self.get(key).is_some()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&QName, &str)> {
        self.attrs.iter().map(|(k, v)| (k, v.as_str()))
    }

    pub fn is_empty(&self) -> bool {
        self.attrs.is_empty()
    }

    pub fn len(&self) -> usize {
        self.attrs.len()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Item {
    pub attrs: Node,
    pub rcvd: Node,
    pub sent: Node,
}
