#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Call(String);

impl Call {
    pub fn new(text: &str) -> Self {
        let value: String = text
            .chars()
            .map(|c| match c {
                '\u{FF01}'..='\u{FF5E}' => char::from_u32(c as u32 - 0xFEE0).unwrap_or(c),
                '\u{3000}' => ' ',
                _ => c,
            })
            .collect();
        Call(value.to_uppercase())
    }

    pub fn value(&self) -> &str {
        &self.0
    }

    pub fn strip(&self) -> &str {
        self.0.split('/').next().unwrap_or_default()
    }

    pub fn valid(&self) -> bool {
        let word =
            |s: &str| !s.is_empty() && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'A'..=b'Z'));
        let (body, port) = match self.0.split_once('/') {
            Some((b, p)) => (b, Some(p)),
            None => (self.0.as_str(), None),
        };
        let (base, tail) = match body.split_once('-') {
            Some((b, t)) => (b, Some(t)),
            None => (body, None),
        };
        word(base) && tail.is_none_or(word) && port.is_none_or(word)
    }
}

impl std::fmt::Display for Call {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rstq(pub i32);

impl Rstq {
    pub fn new(r: i32, s: i32, t: i32) -> Self {
        Rstq(r * 100 + s * 10 + t)
    }

    pub fn valid(&self) -> bool {
        let digits: Vec<i32> = match self.0 {
            10..=99 => vec![self.0 / 10, self.0 % 10],
            100..=999 => vec![self.0 / 100, self.0 / 10 % 10, self.0 % 10],
            _ => return false,
        };
        (1..=5).contains(&digits[0]) && digits[1..].iter().all(|d| (1..=9).contains(d))
    }
}

pub fn valid_watt(watt: &str) -> bool {
    matches!(watt, "P" | "L" | "M" | "H")
}

pub fn mode(text: &str) -> String {
    text.to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calls() {
        assert_eq!(Call::new("ｊａ１ｚｌｏ/1").value(), "JA1ZLO/1");
        assert_eq!(Call::new("JA1ZLO/1").strip(), "JA1ZLO");
        assert!(Call::new("JA1YAA-1/P").valid());
        assert!(!Call::new("JA1 ZLO").valid());
        assert!(!Call::new("JA1ZLO/1/2").valid());
        assert!(!Call::new("").valid());
    }

    #[test]
    fn reports() {
        assert!(Rstq(599).valid());
        assert!(Rstq(59).valid());
        assert!(!Rstq(699).valid());
        assert!(!Rstq(509).valid());
        assert_eq!(Rstq::new(5, 9, 9), Rstq(599));
    }

    #[test]
    fn watts() {
        assert!(valid_watt("H"));
        assert!(!valid_watt("X"));
    }
}
