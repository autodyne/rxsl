use crate::formats::{
    local_time, md_hm, set_band, set_rstq, set_text, set_zlog_mults, set_zlog_rest, some,
    zlog_band_text, zlog_row,
};
use crate::model::{Item, QName};
use crate::text::{
    current_year, decode_sjis, encode_sjis, lines, sjis_head, sjis_tail, slices, NEWLINE,
};
use crate::value::JST;
use crate::{Format, Result};

pub struct ZDos {
    pub offset: i32,
    pub year: Option<i32>,
}

impl Default for ZDos {
    fn default() -> Self {
        ZDos {
            offset: JST,
            year: None,
        }
    }
}

impl Format for ZDos {
    fn kind(&self) -> &'static str {
        "zdos"
    }

    fn name(&self) -> &'static str {
        "zLog (DOS)"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["txt"]
    }

    fn decode(&self, bytes: &[u8]) -> Result<Vec<Item>> {
        let text = decode_sjis(bytes);
        let year = self.year.unwrap_or_else(current_year);
        let mut items = Vec::new();
        let mut head = true;
        let mut nr = 10;
        for line in lines(&text) {
            if head && line.starts_with("mon") {
                nr = if line.len() > LONG.len() - 4 { 31 } else { 10 };
                continue;
            }
            if head && line.trim().is_empty() {
                continue;
            }
            head = false;
            if line.trim().is_empty() {
                continue;
            }
            let c = 24 + 2 * (3 + nr);
            let v = slices(
                line,
                &[
                    0,
                    4,
                    8,
                    13,
                    24,
                    27,
                    24 + 3 + nr,
                    c - nr,
                    c,
                    c + 6,
                    c + 12,
                    c + 17,
                    c + 20,
                ],
            );
            let mut item = Item::default();
            if !v[0].is_empty() {
                let t = md_hm(&v[0], &v[1], &v[2], year, self.offset)?;
                item.attrs.set(QName::qxsl("time"), t.to_iso());
            }
            set_text(&mut item.rcvd, "call", &v[3]);
            set_rstq(&mut item.sent, &v[4])?;
            set_text(&mut item.sent, "code", &v[5]);
            set_rstq(&mut item.rcvd, &v[6])?;
            set_text(&mut item.rcvd, "code", &v[7]);
            set_zlog_mults(&mut item, if v[8].is_empty() { "-" } else { &v[8] }, "-");
            let band = match v[9].as_str() {
                b @ ("0.1G" | "0.4G") => format!("1{b}"),
                b => b.to_string(),
            };
            set_band(&mut item.attrs, &zlog_band_text(&band))?;
            set_text(&mut item.attrs, "mode", &v[10]);
            set_zlog_rest(&mut item, &v[11], &v[12]);
            items.push(item);
        }
        Ok(items)
    }

    fn encode(&self, items: &[Item]) -> Result<Vec<u8>> {
        let long = items
            .last()
            .is_some_and(|i| i.sent.qxsl("code").unwrap_or("").encode_utf16().count() >= 10);
        let (nr, gap) = if long { (31, "") } else { (10, "  ") };
        let mut out = format!("{}{NEWLINE}", if long { LONG } else { SHORT });
        for item in items {
            let t = local_time(item, self.offset)
                .map(|t| format!("{:>3} {:>3} {:02}{:02} ", t.month, t.day, t.hour, t.minute))
                .unwrap_or_else(|| " ".repeat(13));
            let row = zlog_row(item);
            out.push_str(&t);
            out.push_str(&sjis_tail(item.rcvd.qxsl("call").unwrap_or(""), 11));
            out.push_str(&sjis_head(item.sent.qxsl("rstq").unwrap_or(""), 3));
            out.push_str(&sjis_tail(item.sent.qxsl("code").unwrap_or(""), nr));
            out.push_str(&sjis_head(item.rcvd.qxsl("rstq").unwrap_or(""), 3));
            out.push_str(&sjis_tail(item.rcvd.qxsl("code").unwrap_or(""), nr));
            out.push_str(&sjis_head(if row.mul1 == "-" { "" } else { &row.mul1 }, 6));
            out.push_str("  ");
            out.push_str(&sjis_head(&row.band, 4));
            out.push_str("  ");
            out.push_str(&sjis_tail(some(item, "mode"), 3));
            out.push(' ');
            out.push_str(&sjis_tail(&row.points, 2));
            if !some(item, "name").is_empty() {
                out.push_str(gap);
                out.push_str(&format!("%%{}%%", some(item, "name")));
            }
            out.push_str(gap);
            out.push_str(&row.memo);
            out.push_str(NEWLINE);
        }
        Ok(encode_sjis(&out))
    }
}

const SHORT: &str = "mon day time  callsign      sent         rcvd      multi   MHz mode pts memo";
const LONG: &str = "mon day time  callsign      sent                              rcvd                           multi   MHz mode pts memo";
