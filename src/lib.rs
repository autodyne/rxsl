pub mod conv;
pub mod formats;
pub mod model;
pub mod sheets;
pub mod text;
pub mod value;

pub use model::{Item, Node, QName, ADIF, QXSL};
pub use sheets::{Field, Sheet, SheetFormat};
pub use value::{Band, Time};

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
    use formats::*;
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
        Box::new(cbin::CBin),
        Box::new(hbin::HBin),
    ]
}
