use crate::formats::{
    local_time, set_band, set_rstq, set_text, set_zlog_mults, set_zlog_rest, some, zlog_band_text,
    zlog_row,
};
use crate::model::{Item, QName};
use crate::text::{decode_sjis, encode_sjis, lines, sjis_tail, slices, NEWLINE};
use crate::value::JST;
use crate::{Error, Format, Result, Time};

pub struct ZAll {
    pub offset: i32,
}

impl Default for ZAll {
    fn default() -> Self {
        ZAll { offset: JST }
    }
}

impl Format for ZAll {
    fn kind(&self) -> &'static str {
        "zall"
    }

    fn name(&self) -> &'static str {
        "zLog (ALL)"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["all"]
    }

    fn decode(&self, bytes: &[u8]) -> Result<Vec<Item>> {
        let text = decode_sjis(bytes);
        let mut items = Vec::new();
        let mut head = true;
        for line in lines(&text) {
            if head
                && (line.trim().is_empty() || line.starts_with("zLog") || line.starts_with("Date"))
            {
                continue;
            }
            head = false;
            if line.trim().is_empty() {
                continue;
            }
            let v = slices(line, &[0, 17, 30, 34, 42, 46, 54, 60, 66, 71, 76, 79]);
            let mut item = Item::default();
            if !v[0].is_empty() {
                item.attrs
                    .set(QName::qxsl("time"), time(&v[0], self.offset)?.to_iso());
            }
            set_text(&mut item.rcvd, "call", &v[1]);
            set_rstq(&mut item.sent, &v[2])?;
            set_text(&mut item.sent, "code", &v[3]);
            set_rstq(&mut item.rcvd, &v[4])?;
            set_text(&mut item.rcvd, "code", &v[5]);
            set_zlog_mults(&mut item, &v[6], &v[7]);
            set_band(&mut item.attrs, &zlog_band_text(&v[8]))?;
            set_text(&mut item.attrs, "mode", &v[9]);
            set_zlog_rest(&mut item, &v[10], &v[11]);
            items.push(item);
        }
        Ok(items)
    }

    fn encode(&self, items: &[Item]) -> Result<Vec<u8>> {
        let mut out = format!("zLog for Windows {NEWLINE}");
        for item in items {
            let t = local_time(item, self.offset)
                .map(|t| {
                    format!(
                        "{:04}/{:02}/{:02} {:02}:{:02} ",
                        t.year, t.month, t.day, t.hour, t.minute
                    )
                })
                .unwrap_or_else(|| " ".repeat(17));
            let row = zlog_row(item);
            out.push_str(&t);
            out.push_str(&sjis_tail(item.rcvd.qxsl("call").unwrap_or(""), 13));
            out.push_str(&sjis_tail(item.sent.qxsl("rstq").unwrap_or(""), 4));
            out.push_str(&sjis_tail(item.sent.qxsl("code").unwrap_or(""), 8));
            out.push_str(&sjis_tail(item.rcvd.qxsl("rstq").unwrap_or(""), 4));
            out.push_str(&sjis_tail(item.rcvd.qxsl("code").unwrap_or(""), 8));
            out.push_str(&sjis_tail(&row.mul1, 6));
            out.push_str(&sjis_tail(&row.mul2, 6));
            out.push_str(&sjis_tail(&row.band, 5));
            out.push_str(&sjis_tail(some(item, "mode"), 5));
            out.push_str(&sjis_tail(&row.points, 3));
            if !some(item, "name").is_empty() {
                out.push_str(&sjis_tail(&format!("%%{}%%", some(item, "name")), 19));
            }
            out.push_str(&row.memo);
            out.push_str(NEWLINE);
        }
        Ok(encode_sjis(&out))
    }
}

fn time(text: &str, offset: i32) -> Result<Time> {
    let bad = || Error::new(format!("malformed time: {text}"));
    let (d, hm) = text.split_once(' ').ok_or_else(bad)?;
    Time::parse_iso(&format!("{}T{}", d.replace('/', "-"), hm.trim()))
        .or_else(|_| Time::parse_iso(&format!("{}T{}Z", d.replace('/', "-"), hm.trim())))
        .map(|t| Time { offset, ..t })
        .map_err(|_| bad())
}
