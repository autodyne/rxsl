use crate::{Error, Result};

pub const NEWLINE: &str = "\r\n";

pub fn decode_sjis(bytes: &[u8]) -> String {
    encoding_rs::SHIFT_JIS.decode(bytes).0.into_owned()
}

pub fn encode_sjis(text: &str) -> Vec<u8> {
    encoding_rs::SHIFT_JIS.encode(text).0.into_owned()
}

pub fn decode_ascii(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| if b < 0x80 { b as char } else { '\u{FFFD}' })
        .collect()
}

pub fn encode_ascii(text: &str) -> Vec<u8> {
    text.chars()
        .map(|c| if c.is_ascii() { c as u8 } else { b'?' })
        .collect()
}

pub fn lines(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        match rest.find(['\r', '\n']) {
            Some(p) => {
                out.push(&rest[..p]);
                let skip = if rest[p..].starts_with("\r\n") { 2 } else { 1 };
                rest = &rest[p + skip..];
            }
            None => {
                out.push(rest);
                rest = "";
            }
        }
    }
    out
}

pub fn columns(line: &str, cols: &[usize]) -> Result<Vec<String>> {
    let chars: Vec<char> = line.chars().collect();
    let n = chars.len();
    let mut out = Vec::with_capacity(cols.len() - 1);
    for i in 1..cols.len() {
        let head = cols[i - 1].min(n);
        let last = (cols[i] - 1).min(n);
        if i + 1 < cols.len() && last < n && chars[last] != ' ' {
            return Err(Error::new(format!("malformed record: {line}")));
        }
        let field: String = chars[head..last].iter().collect();
        out.push(field.trim().to_string());
    }
    Ok(out)
}

pub fn slices(line: &str, cols: &[usize]) -> Vec<String> {
    let b = encode_sjis(line);
    let cut = |i: usize| i.min(b.len());
    let mut out: Vec<String> = cols
        .windows(2)
        .map(|w| decode_sjis(&b[cut(w[0])..cut(w[1])]).trim().to_string())
        .collect();
    out.push(
        decode_sjis(&b[cut(cols[cols.len() - 1])..])
            .trim()
            .to_string(),
    );
    out
}

pub fn sjis_tail(text: &str, size: usize) -> String {
    let b = sjis_cut(text, size);
    let n = b.len();
    decode_sjis(&b) + &" ".repeat(size - n)
}

pub fn sjis_head(text: &str, size: usize) -> String {
    let b = encode_sjis(text);
    match b.len() >= size {
        true => decode_sjis(&b[b.len() - size..]),
        false => " ".repeat(size - b.len()) + text,
    }
}

pub fn pad_tail(text: &str, size: usize) -> String {
    let t: String = text.chars().take(size).collect();
    let n = t.chars().count();
    t + &" ".repeat(size - n)
}

pub fn pad_head(text: &str, size: usize) -> String {
    let t: String = text.chars().take(size).collect();
    let n = t.chars().count();
    " ".repeat(size - n) + &t
}

pub fn fill_tail(text: &str, size: usize) -> String {
    let n = text.chars().count();
    text.to_string() + &" ".repeat(size.saturating_sub(n))
}

pub fn fill_head(text: &str, size: usize) -> String {
    let n = text.chars().count();
    " ".repeat(size.saturating_sub(n)) + text
}

pub fn right(text: &str, size: usize) -> String {
    pad_head(text, size)
}

pub fn sjis_cut(text: &str, size: usize) -> Vec<u8> {
    let mut out = Vec::new();
    for c in text.chars() {
        let b = encode_sjis(c.encode_utf8(&mut [0; 4]));
        if out.len() + b.len() > size {
            break;
        }
        out.extend(b);
    }
    out
}

pub fn sjis_field(text: &str, size: usize, fill: u8) -> Vec<u8> {
    let mut b = sjis_cut(text, size);
    b.resize(size, fill);
    b
}

pub fn sjis_until_nul(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    decode_sjis(&bytes[..end])
}

pub fn current_year() -> i32 {
    crate::value::Time::now(crate::value::JST).year
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_columns() {
        let v = columns("ab   cd  e", &[0, 5, 9, 20]).unwrap();
        assert_eq!(v, ["ab", "cd", "e"]);
        assert!(columns("abcdefgh", &[0, 3, 9]).is_err());
        assert_eq!(lines("a\r\nb\nc\rd"), ["a", "b", "c", "d"]);
        assert_eq!(pad_tail("abc", 2), "ab");
        assert_eq!(pad_head("7", 3), "  7");
        assert_eq!(fill_tail("abc", 2), "abc");
        assert_eq!(fill_tail("a", 3), "a  ");
        assert_eq!(fill_head("abcdef", 5), "abcdef");
        assert_eq!(fill_head("7", 3), "  7");
        assert_eq!(
            slices("ab 100110-  x y", &[0, 3, 9, 12]),
            ["ab", "100110", "-", "x y"]
        );
        assert_eq!(sjis_tail("移動abc", 5), "移動a");
        assert_eq!(sjis_tail("ab", 3), "ab ");
        assert_eq!(sjis_head("10.4G", 4), "0.4G");
        assert_eq!(sjis_head("7", 3), "  7");
    }
}
