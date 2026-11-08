pub mod conv;
pub mod draft;
pub mod local;
pub mod model;
pub mod ruler;
pub mod sheet;
pub mod sqlite;
pub mod table;
pub mod text;
pub mod value;

pub use model::{Item, Node, QName, ADIF, QXSL};
pub use sheet::{Field, Sheet, SheetFormat};
pub use value::{Band, Date, Time, Weekday};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl Error {
    pub fn new(message: impl Into<String>) -> Self {
        Error(message.into())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub trait Format: Send + Sync {
    fn kind(&self) -> &'static str;

    fn name(&self) -> &'static str;

    fn extensions(&self) -> &'static [&'static str];

    fn decode(&self, bytes: &[u8]) -> Result<Vec<Item>>;

    fn encode(&self, items: &[Item]) -> Result<Vec<u8>>;

    fn read(&self, bytes: &[u8]) -> Result<Vec<Item>> {
        Ok(self
            .decode(bytes)?
            .iter()
            .map(|i| conv::normalize(i, self.kind()))
            .collect())
    }

    fn write(&self, items: &[Item]) -> Result<Vec<u8>> {
        let items: Vec<Item> = items
            .iter()
            .map(|i| conv::transform(i, self.kind()))
            .collect();
        self.encode(&items)
    }
}

pub fn formats() -> Vec<Box<dyn Format>> {
    use table::*;
    vec![
        Box::new(qxml::Qxml),
        Box::new(adis::Adis),
        Box::new(adxs::Adxs),
        Box::new(cqww::Cqww),
        Box::new(jarl::Jarl::default()),
        Box::new(jswl::Jswl::default()),
        Box::new(zall::ZAll::default()),
        Box::new(zdos::ZDos::default()),
        Box::new(ctxt::CTxt::default()),
        Box::new(zbin::ZBin),
        Box::new(znew::ZNew),
        Box::new(cbin::CBin),
        Box::new(hbin::HBin),
        Box::new(s3db::S3db),
    ]
}

pub fn format(name: &str) -> Option<Box<dyn Format>> {
    let all = formats();
    let at = all
        .iter()
        .position(|f| f.kind() == name)
        .or_else(|| all.iter().position(|f| f.name() == name))
        .or_else(|| {
            all.iter()
                .position(|f| f.extensions().iter().any(|e| e.eq_ignore_ascii_case(name)))
        })?;
    all.into_iter().nth(at)
}

pub fn decode(bytes: &[u8]) -> Result<Vec<Item>> {
    let mut causes = Vec::new();
    for f in formats() {
        match f.decode(bytes) {
            Ok(items) => return Ok(items),
            Err(e) => causes.push(format!(" [{}]: {e}", f.name())),
        }
    }
    Err(Error::new(causes.join("\n")))
}

pub fn encode(items: &[Item]) -> Result<Vec<u8>> {
    table::qxml::Qxml.encode(items)
}
