use std::collections::HashMap;

use crate::model::{Item, QName};
use crate::sqlite::{columns, index_keys, write, Object, Reader, Table, Value};
use crate::{Band, Error, Format, Result, Time};

pub struct S3db;

const DXLOG: &str = "CREATE TABLE `DXLOG`
(
  `TS` DATETIME NOT NULL,
  `Call` VARCHAR(15) NOT NULL,
  `Freq` DOUBLE NULL,
  `QSXFreq` DOUBLE NULL DEFAULT 0,
  `Mode` VARCHAR(6),
  `ContestName` VARCHAR(10) DEFAULT 'NORMAL',
  `SNT` VARCHAR(10),
  `RCV` VARCHAR(15),
  `CountryPrefix` VARCHAR(8) DEFAULT '',
  `StationPrefix` VARCHAR(15) DEFAULT '',
  `QTH` VARCHAR(25) DEFAULT '',
  `Name` VARCHAR(20) DEFAULT '',
  `Comment` VARCHAR(60) DEFAULT '',
  `NR` INTEGER DEFAULT 0,
  `Sect` VARCHAR(8) DEFAULT '',
  `Prec` VARCHAR(1) DEFAULT '',
  `CK` TINYINT DEFAULT 0,
  `ZN` TINYINT DEFAULT 0,
  `SentNr` INTEGER DEFAULT 0,
  `Points` INTEGER DEFAULT 0,
  `IsMultiplier1` TINYINT DEFAULT 0,
  `IsMultiplier2` INTEGER DEFAULT 0,
  `Power` VARCHAR(8),
  `Band` FLOAT NULL DEFAULT 0,
  `WPXPrefix` VARCHAR(8),
  `Exchange1` VARCHAR(20),
  `RadioNR` TINYINT DEFAULT 1,
  `ContestNR` INTEGER,
  `isMultiplier3` INTEGER,
  `MiscText` VARCHAR(20),
  `IsRunQSO` TINYINT(1) DEFAULT 0,
  `ContactType` VARCHAR(1),
  `Run1Run2` TINYINT NOT NULL,
  `GridSquare` VARCHAR(6),
  `Operator` VARCHAR(20),
  `Continent` VARCHAR(2),
  `RoverLocation` VARCHAR(10),
  `RadioInterfaced` INTEGER,
  `NetworkedCompNr` INTEGER, NetBiosName varchar (255), IsOriginal Boolean,
  [ID] TEXT(16) NOT NULL DEFAULT '0000000000000000', [CLAIMEDQSO] INTEGER DEFAULT 1,
  PRIMARY KEY (`TS`, `Call`)
)";

const ID_INDEX: &str = "CREATE UNIQUE INDEX [ID_INDEX] ON [DXLOG]([ID] COLLATE [BINARY] ASC)";

const SEQUENCE: &str = "CREATE TABLE sqlite_sequence(name,seq)";

const CONTEST_INSTANCE: &str = "CREATE TABLE `ContestInstance`
(
  `ContestID` INTEGER PRIMARY KEY AUTOINCREMENT,
  `ContestName` VARCHAR(30) NOT NULL,
  `StartDate` DATETIME,
  `SentExchange` VARCHAR(50),
  `ContestNR` INTEGER
)";

const CONTEST: &str = "DX";

const CONTEST_NR: i64 = 1;

fn table(db: &Reader, name: &str) -> Result<Table> {
    db.table(name)?
        .ok_or_else(|| Error::new(format!("s3db has no {name} table")))
}

fn cell(t: &Table, row: &[Value], name: &str) -> Value {
    t.column(name)
        .and_then(|i| row.get(i).cloned())
        .unwrap_or(Value::Null)
}

fn text(t: &Table, row: &[Value], name: &str) -> Option<String> {
    cell(t, row, name).text()
}

fn ts(text: &str) -> Result<Time> {
    let bad = || Error::new(format!("malformed time: {text}"));
    let (date, clock) = text.trim().split_once(' ').ok_or_else(bad)?;
    Time::parse_iso(&format!("{date}T{clock}Z")).map_err(|_| bad())
}

fn rst(text: &str) -> Option<String> {
    let digits: String = text.chars().take_while(char::is_ascii_digit).collect();
    match digits.len() {
        2 | 3 => Some(digits),
        n if n > 3 && text.starts_with('5') => Some(digits[..3].to_string()),
        _ => None,
    }
}

fn put(node: &mut crate::Node, local: &str, value: Option<String>) {
    if let Some(v) = value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
    {
        node.set(QName::qxsl(local), v);
    }
}

impl Format for S3db {
    fn kind(&self) -> &'static str {
        "s3db"
    }

    fn name(&self) -> &'static str {
        "N1MM Logger+ (s3db)"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["s3db"]
    }

    fn decode(&self, bytes: &[u8]) -> Result<Vec<Item>> {
        let db = Reader::open(bytes)?;
        let log = table(&db, "DXLOG")?;
        let contests = table(&db, "ContestInstance")?;
        let mut instances: HashMap<i64, (Option<String>, Option<String>)> = HashMap::new();
        for r in &contests.rows {
            if let Some(nr) = cell(&contests, r, "ContestNR").int() {
                let name = text(&contests, r, "ContestName");
                let sent = text(&contests, r, "SentExchange");
                instances.entry(nr).or_insert((name, sent));
            }
        }
        let mut rows: Vec<&Vec<Value>> = log.rows.iter().collect();
        rows.sort_by_key(|r| text(&log, r, "TS").unwrap_or_default());
        let mut items = Vec::new();
        for r in rows {
            let get = |name: &str| text(&log, r, name);
            let nr = cell(&log, r, "ContestNR").int();
            let (contest, exchange) = nr
                .and_then(|n| instances.get(&n).cloned())
                .unwrap_or((None, None));
            if contest.as_deref() == Some("DELETEDQS") {
                continue;
            }
            let time = get("TS").ok_or_else(|| Error::new("s3db: QSO without TS"))?;
            let mut item = Item::default();
            item.attrs.set(QName::qxsl("time"), ts(&time)?.to_iso());
            if let Some(b) = cell(&log, r, "Freq").real().filter(|f| *f > 0.0) {
                if let Ok(b) = Band::parse(&format!("{}kHz", (b * 1000.0).round() / 1000.0)) {
                    item.attrs.set(QName::qxsl("band"), b.khz());
                }
            }
            put(&mut item.attrs, "mode", get("Mode"));
            put(&mut item.attrs, "name", get("Operator"));
            put(&mut item.attrs, "note", get("Comment"));
            put(&mut item.rcvd, "call", get("Call"));
            put(&mut item.rcvd, "rstq", get("RCV").as_deref().and_then(rst));
            let number = cell(&log, r, "NR").int().filter(|n| *n > 0);
            let code = [get("Exchange1"), get("Sect"), number.map(|n| n.to_string())]
                .into_iter()
                .flatten()
                .find(|c| !c.trim().is_empty());
            put(&mut item.rcvd, "code", code);
            put(&mut item.sent, "call", get("StationPrefix"));
            put(&mut item.sent, "rstq", get("SNT").as_deref().and_then(rst));
            let sent_nr = cell(&log, r, "SentNr").int().filter(|n| *n > 0);
            let sent = exchange
                .filter(|e| !e.trim().is_empty())
                .or_else(|| sent_nr.map(|n| n.to_string()));
            put(&mut item.sent, "code", sent);
            items.push(item);
        }
        Ok(items)
    }

    fn encode(&self, items: &[Item]) -> Result<Vec<u8>> {
        let sent = items.iter().find_map(|i| i.sent.qxsl("code")).unwrap_or("");
        let start = items
            .iter()
            .filter_map(|i| i.attrs.qxsl("time").and_then(|t| Time::parse_iso(t).ok()))
            .map(|t| t.at_offset(0))
            .min_by_key(Time::unix);
        let date = start.map(|t| format!("{:04}-{:02}-{:02} 00:00:00", t.year, t.month, t.day));
        let contest = vec![
            Value::Null,
            Value::Text(CONTEST.into()),
            date.map_or(Value::Null, Value::Text),
            Value::Text(sent.into()),
            Value::Int(CONTEST_NR),
        ];
        let names: Vec<String> = columns(DXLOG).into_iter().map(|(c, _)| c).collect();
        let mut rows = Vec::new();
        for (i, item) in items.iter().enumerate() {
            let time = item
                .attrs
                .qxsl("time")
                .and_then(|t| Time::parse_iso(t).ok())
                .ok_or_else(|| Error::new("s3db needs the time of every QSO"))?
                .at_offset(0);
            let ts = format!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                time.year, time.month, time.day, time.hour, time.minute, time.second
            );
            let khz = item
                .attrs
                .qxsl("band")
                .and_then(|b| Band::parse(b).ok())
                .map(|b| b.0 as f64 / 1000.0);
            let s = |n: &crate::Node, k: &str| Value::Text(n.qxsl(k).unwrap_or("").into());
            let sent_nr: i64 = item.sent.qxsl("code").unwrap_or("").parse().unwrap_or(0);
            let real = |v: Option<f64>| v.map_or(Value::Null, Value::Real);
            let given: HashMap<&str, Value> = HashMap::from([
                ("TS", Value::Text(ts)),
                ("Call", s(&item.rcvd, "call")),
                ("Freq", real(khz)),
                ("QSXFreq", real(khz)),
                ("Mode", s(&item.attrs, "mode")),
                ("ContestName", Value::Text(CONTEST.into())),
                ("SNT", s(&item.sent, "rstq")),
                ("RCV", s(&item.rcvd, "rstq")),
                ("StationPrefix", s(&item.sent, "call")),
                ("Comment", s(&item.attrs, "note")),
                ("SentNr", Value::Int(sent_nr)),
                ("Band", real(khz.map(|k| k / 1000.0))),
                ("Exchange1", s(&item.rcvd, "code")),
                ("ContestNR", Value::Int(CONTEST_NR)),
                ("Run1Run2", Value::Int(1)),
                ("Operator", s(&item.attrs, "name")),
                ("IsOriginal", Value::Int(1)),
                ("ID", Value::Text(format!("{:032x}", i + 1))),
            ]);
            let row = names
                .iter()
                .map(|n| given.get(n.as_str()).cloned().unwrap_or_else(|| default(n)))
                .collect();
            rows.push((i as i64 + 1, row));
        }
        let at = |name: &str| names.iter().position(|n| n == name).unwrap_or(0);
        let primary = index_keys(&rows, &[at("TS"), at("Call")])?;
        let ids = index_keys(&rows, &[at("ID")])?;
        let object = |kind, name: &str, sql: Option<&str>, rows| Object {
            kind,
            name: name.into(),
            table: name.into(),
            sql: sql.map(String::from),
            rows,
        };
        let mut objects = vec![
            object("table", "DXLOG", Some(DXLOG), rows),
            object("index", "sqlite_autoindex_DXLOG_1", None, primary),
            object("index", "ID_INDEX", Some(ID_INDEX), ids),
            object(
                "table",
                "ContestInstance",
                Some(CONTEST_INSTANCE),
                vec![(1, contest)],
            ),
            object(
                "table",
                "sqlite_sequence",
                Some(SEQUENCE),
                vec![(
                    1,
                    vec![Value::Text("ContestInstance".into()), Value::Int(1)],
                )],
            ),
        ];
        for o in &mut objects[1..3] {
            o.table = "DXLOG".into();
        }
        write(&objects)
    }
}

fn default(column: &str) -> Value {
    match column {
        "QSXFreq" | "NR" | "CK" | "ZN" | "SentNr" | "Points" | "IsMultiplier1"
        | "IsMultiplier2" | "IsRunQSO" => Value::Int(0),
        "Band" => Value::Real(0.0),
        "RadioNR" | "CLAIMEDQSO" => Value::Int(1),
        "ContestName" => Value::Text("NORMAL".into()),
        "CountryPrefix" | "StationPrefix" | "QTH" | "Name" | "Comment" | "Sect" | "Prec" => {
            Value::Text(String::new())
        }
        "ID" => Value::Text("0000000000000000".into()),
        _ => Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rst_is_read_from_the_head_of_the_report() {
        assert_eq!(rst("599"), Some("599".into()));
        assert_eq!(rst("599100"), Some("599".into()));
        assert_eq!(rst("59"), Some("59".into()));
        assert_eq!(rst(""), None);
    }

    #[test]
    fn a_written_log_is_read_back() {
        let mut item = Item::default();
        item.attrs
            .set(QName::qxsl("time"), "2024-07-03T23:52:37+09:00");
        item.attrs.set(QName::qxsl("band"), "7000");
        item.attrs.set(QName::qxsl("mode"), "CW");
        item.rcvd.set(QName::qxsl("call"), "JA1YGX");
        item.rcvd.set(QName::qxsl("rstq"), "599");
        item.rcvd.set(QName::qxsl("code"), "100105H");
        item.sent.set(QName::qxsl("call"), "JA1ZLO");
        item.sent.set(QName::qxsl("rstq"), "599");
        item.sent.set(QName::qxsl("code"), "100110H");
        let bytes = S3db.encode(&[item]).unwrap();
        let back = S3db.decode(&bytes).unwrap();
        assert_eq!(back.len(), 1);
        let b = &back[0];
        assert_eq!(b.attrs.qxsl("time"), Some("2024-07-03T14:52:37Z"));
        assert_eq!(b.attrs.qxsl("band"), Some("7000"));
        assert_eq!(b.rcvd.qxsl("code"), Some("100105H"));
        assert_eq!(b.sent.qxsl("code"), Some("100110H"));
        assert!(S3db.decode(b"not sqlite").is_err());
    }
}
