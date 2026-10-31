use crate::sheets::{Field, Sheet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Version {
    R1,
    #[default]
    R2,
}

impl Version {
    pub fn label(self) -> &'static str {
        match self {
            Version::R1 => "R1.0",
            Version::R2 => "R2.1",
        }
    }

    pub fn parse(text: &str) -> Self {
        match text.trim().starts_with("R1") {
            true => Version::R1,
            false => Version::R2,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Score {
    pub band: String,
    pub qsos: u64,
    pub points: u64,
    pub mults: u64,
}

impl Score {
    fn text(&self) -> String {
        format!("{},{},{}", self.qsos, self.points, self.mults)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Summary {
    pub version: Version,
    pub contest: String,
    pub category_code: String,
    pub category_name: String,
    pub callsign: String,
    pub op_callsign: String,
    pub scores: Vec<Score>,
    pub coeff: Option<String>,
    pub total_score: String,
    pub address: String,
    pub tel: String,
    pub name: String,
    pub email: String,
    pub license_class: String,
    pub power: String,
    pub power_type: String,
    pub op_place: String,
    pub power_supply: String,
    pub equipment: String,
    pub license_date: Option<String>,
    pub age: Option<String>,
    pub comments: String,
    pub operators: Vec<String>,
    pub club_number: String,
    pub club_name: String,
    pub oath: String,
    pub date: String,
    pub signature: String,
}

pub const NEWCOMER: &str = "PN";

pub const SENIOR_JUNIOR: &str = "XS,CS,SOSV,SOJR";

pub const OATH_JARL: &str = "私は、JARL制定のコンテスト規約および電波法令にしたがい運用した結果、ここに提出するサマリーシートおよびログシートなどが事実と相違ないものであることを、私の名誉において誓います。";

pub const OATH_OTHER: &str = "私は、主催者制定のコンテスト規約および電波法令にしたがい運用した結果、ここに提出するサマリーシートおよびログシートなどが事実と相違ないものであることを、私の名誉において誓います。";

pub const ADDRESS: &str = "〒";

pub const MEASURED: &str = "実測出力";

pub const RATED: &str = "定格出力";

pub const NO_LICENSE_DATE: (i32, u32, u32) = (2000, 1, 1);

pub fn oath(jarl: bool) -> &'static str {
    match jarl {
        true => OATH_JARL,
        false => OATH_OTHER,
    }
}

pub fn power_type(measured: bool) -> &'static str {
    match measured {
        true => MEASURED,
        false => RATED,
    }
}

pub fn date_text((y, m, d): (i32, u32, u32)) -> String {
    format!("{y}年{m}月{d}日")
}

pub fn license_date_text((y, m, d): (i32, u32, u32)) -> String {
    format!("{y}年{m:02}月{d:02}日")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rules {
    pub newcomer: String,
    pub senior_junior: String,
}

impl Default for Rules {
    fn default() -> Self {
        Rules {
            newcomer: NEWCOMER.into(),
            senior_junior: SENIOR_JUNIOR.into(),
        }
    }
}

impl Rules {
    pub fn is_newcomer(&self, category: &str) -> bool {
        self.newcomer.split(',').any(|c| c == category)
    }

    pub fn is_senior_junior(&self, category: &str) -> bool {
        self.senior_junior.split(',').any(|c| c == category)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Missing {
    Signature,
    LicenseDate,
    Age,
}

pub fn memo(text: &str) -> String {
    let mut lines: Vec<&str> = text.lines().collect();
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines.iter().map(|l| format!("{l}\n")).collect()
}

pub fn csv(items: &[String]) -> String {
    let quote =
        |s: &String| match s.is_empty() || s.chars().any(|c| c <= ' ' || c == ',' || c == '"') {
            true => format!("\"{}\"", s.replace('"', "\"\"")),
            false => s.clone(),
        };
    items.iter().map(quote).collect::<Vec<_>>().join(",")
}

pub fn parse_csv(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = text.trim().chars().peekable();
    while chars.peek().is_some() {
        let mut field = String::new();
        if chars.next_if_eq(&'"').is_some() {
            while let Some(c) = chars.next() {
                if c == '"' && chars.next_if_eq(&'"').is_none() {
                    break;
                }
                field.push(c);
            }
            while chars.next_if(|c| *c != ',').is_some() {}
        } else {
            while let Some(c) = chars.next_if(|c| *c != ',') {
                field.push(c);
            }
        }
        out.push(field.trim().to_string());
        if chars.next_if_eq(&',').is_some() && chars.peek().is_none() {
            out.push(String::new());
        }
    }
    out
}

impl Summary {
    pub fn total(&self) -> Score {
        let mut t = Score {
            band: "TOTAL".into(),
            ..Score::default()
        };
        for s in &self.scores {
            t.qsos += s.qsos;
            t.points += s.points;
            t.mults += s.mults;
        }
        t
    }

    pub fn set_operators<'a>(&mut self, calls: impl IntoIterator<Item = &'a str>) {
        self.operators.clear();
        for c in calls.into_iter().map(str::trim) {
            if !c.is_empty() && !self.operators.iter().any(|o| o == c) {
                self.operators.push(c.to_string());
            }
        }
    }

    pub fn set_category(
        &mut self,
        rules: &Rules,
        license_date: Option<(i32, u32, u32)>,
        age: &str,
    ) {
        let r2 = self.version == Version::R2;
        self.license_date = (r2 && rules.is_newcomer(&self.category_code))
            .then(|| license_date_text(license_date.unwrap_or(NO_LICENSE_DATE)));
        self.age = (r2 && rules.is_senior_junior(&self.category_code)).then(|| age.to_string());
    }

    pub fn missing(&self, rules: &Rules) -> Vec<Missing> {
        let mut out = Vec::new();
        if self.signature.is_empty() {
            out.push(Missing::Signature);
        }
        if self.version == Version::R2 {
            let none = license_date_text(NO_LICENSE_DATE);
            let date = self.license_date.as_deref().unwrap_or(&none);
            if rules.is_newcomer(&self.category_code) && date == none {
                out.push(Missing::LicenseDate);
            }
            let age = self.age.as_deref().unwrap_or("");
            if rules.is_senior_junior(&self.category_code) && age.is_empty() {
                out.push(Missing::Age);
            }
        }
        out
    }

    pub fn to_sheet(&self) -> Sheet {
        let mut f = Vec::new();
        let mut tag = |k: &str, v: &str| f.push(Field::new(k, v));
        let r1 = self.version == Version::R1;
        tag("CONTESTNAME", &self.contest);
        tag("CATEGORYCODE", &self.category_code);
        if r1 {
            tag("CATEGORYNAME", &self.category_name);
        }
        tag("CALLSIGN", &self.callsign);
        tag("OPCALLSIGN", &self.op_callsign);
        if r1 {
            for s in self.scores.iter().chain([&self.total()]) {
                f.push(Field::new("SCORE", s.text()).with("BAND", &s.band));
            }
            self.coeff_field(&mut f);
        }
        let mut tag = |k: &str, v: &str| f.push(Field::new(k, v));
        tag("TOTALSCORE", &self.total_score);
        tag("ADDRESS", &memo(&self.address));
        if r1 {
            tag("TEL", &self.tel);
            tag("NAME", &self.name);
            tag("EMAIL", &self.email);
            tag("LICENSECLASS", &self.license_class);
            tag("POWER", &self.power);
            tag("POWERTYPE", &self.power_type);
        } else {
            tag("NAME", &self.name);
            tag("TEL", &self.tel);
            tag("EMAIL", &self.email);
            tag("POWER", &self.power);
        }
        if !r1 {
            self.coeff_field(&mut f);
        }
        let mut tag = |k: &str, v: &str| f.push(Field::new(k, v));
        tag("OPPLACE", &self.op_place);
        tag("POWERSUPPLY", &self.power_supply);
        if r1 {
            tag("EQUIPMENT", &memo(&self.equipment));
        } else {
            if let Some(d) = &self.license_date {
                tag("LICENSEDATE", d);
            }
            if let Some(a) = &self.age {
                tag("AGE", a);
            }
        }
        tag("COMMENTS", &memo(&self.comments));
        if !self.operators.is_empty() {
            tag("MULTIOPLIST", &csv(&self.operators));
        }
        tag("REGCLUBNUMBER", &self.club_number);
        if r1 {
            tag("REGCLUBNAME", &self.club_name);
        }
        tag("OATH", &memo(&self.oath));
        tag("DATE", &self.date);
        tag("SIGNATURE", &self.signature);
        Sheet {
            version: self.version.label().into(),
            fields: f,
            ..Sheet::default()
        }
    }

    fn coeff_field(&self, f: &mut Vec<Field>) {
        if let Some(c) = &self.coeff {
            f.push(Field::new("FDCOEFF", c));
        }
    }

    pub fn from_sheet(sheet: &Sheet) -> Self {
        let get = |k: &str| sheet.get(k).unwrap_or("").to_string();
        let opt = |k: &str| sheet.get(k).map(str::to_string);
        Summary {
            version: Version::parse(&sheet.version),
            contest: get("CONTESTNAME"),
            category_code: get("CATEGORYCODE"),
            category_name: get("CATEGORYNAME"),
            callsign: get("CALLSIGN"),
            op_callsign: get("OPCALLSIGN"),
            scores: sheet
                .all("SCORE")
                .filter_map(|f| {
                    let band = f.attr("BAND")?;
                    let v: Vec<u64> = f
                        .value
                        .split(',')
                        .map(|n| n.trim().parse().unwrap_or(0))
                        .collect();
                    (!band.eq_ignore_ascii_case("TOTAL")).then(|| Score {
                        band: band.to_string(),
                        qsos: v.first().copied().unwrap_or(0),
                        points: v.get(1).copied().unwrap_or(0),
                        mults: v.get(2).copied().unwrap_or(0),
                    })
                })
                .collect(),
            coeff: opt("FDCOEFF"),
            total_score: get("TOTALSCORE"),
            address: get("ADDRESS"),
            tel: get("TEL"),
            name: get("NAME"),
            email: get("EMAIL"),
            license_class: get("LICENSECLASS"),
            power: get("POWER"),
            power_type: get("POWERTYPE"),
            op_place: get("OPPLACE"),
            power_supply: get("POWERSUPPLY"),
            equipment: get("EQUIPMENT"),
            license_date: opt("LICENSEDATE"),
            age: opt("AGE"),
            comments: get("COMMENTS"),
            operators: parse_csv(&get("MULTIOPLIST")),
            club_number: get("REGCLUBNUMBER"),
            club_name: get("REGCLUBNAME"),
            oath: get("OATH"),
            date: get("DATE"),
            signature: get("SIGNATURE"),
        }
    }
}
