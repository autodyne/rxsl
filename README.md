# RxSL

![image](https://img.shields.io/badge/Rust-2021-red.svg)
![image](https://img.shields.io/badge/license-MIT-darkblue.svg)

RxSL reads and writes amateur-radio log files.
It is the Rust version of [QxSL](https://github.com/autodyne/qxsl).

```mermaid
flowchart LR
  F1[ADIF, Cabrillo, JARL, QXML, zLog, CTESTWIN, HAMLOG] -->|decode| N[normalize]
  N --> M[(items)]
  M --> T[transform]
  T -->|encode| F2[any format]
```

Every format meets in one model, so fields a format does not know survive a round trip.

## Model

```mermaid
classDiagram
  class Item {
    time
    band
    mode
    points
    newmult
    invalid
  }
  class Received {
    call
    rstq
    code
  }
  class Sent {
    call
    rstq
    code
  }
  Item *-- Received : rcvd
  Item *-- Sent : sent
```

Attributes are namespaced as in QXML: `qxsl.org` for the common ones and `adif.org` for ADIF fields.
The sent `call` is kept only by formats that have a field for it.

## Formats

| Module | Format |
|---|---|
| `adis`, `adxs` | ADIF (ADI, ADX) |
| `cqww` | Cabrillo, with zLog's header and columns |
| `jarl`, `jswl` | JARL electronic log, JARL SWL log |
| `qxml` | QXML |
| `zall`, `zdos` | zLog ALL and DOS text, in zLog's columns |
| `ctxt`, `cbin` | CTESTWIN text and LG8 |
| `hbin` | Turbo HAMLOG HDB |

```mermaid
flowchart LR
  Note[note] --> ADIF1[ADIF COMMENT]
  Code[non-numeric exchange] --> ADIF2[ADIF SRX_STRING, STX_STRING]
  Flags[points, newmult, invalid] --> JARL[JARL electronic log]
  Score[points, newmult] --> ZLOG[zLog ALL and DOS]
  Exact[exact frequency] --> ZMEMO[zLog memo]
  Invalid[invalid] --> CBR[Cabrillo X-QSO]
  Freq[10.4 GHz] --> ADIF3[ADIF FREQ and BAND]
```

Cabrillo and CTESTWIN LG8 cannot tell 10.4 GHz from 10 GHz.

## Sheets

A summary sheet wraps a log: ordered fields, then the log bytes of any format.

```mermaid
flowchart LR
  H[contest data] --> S[Summary]
  S -->|to_sheet| SH[(Sheet: fields + table)]
  L[items] -->|embed with a Format| SH
  SH -->|JarlSheet| J["SUMMARYSHEET + LOGSHEET (Shift_JIS)"]
  SH -->|Cab3Sheet| C["START-OF-LOG + QSO lines (ASCII)"]
  J & C -->|unpack| L2[items]
```

| Module | Sheet |
|---|---|
| `sheets::jarl` | JARL summary sheet R1.0 and R2.1, log in `<LOGSHEET>` |
| `sheets::cab3` | Cabrillo 3.0 header, log as `QSO:` lines |
| `sheets::summary` | JARL fields, score rows, tag order, category rules |

`cab3::header` writes every standard tag in order, empty or not.
Characters Shift_JIS or ASCII cannot hold become `?`; `Cab3Sheet { utf8: true }` writes UTF-8, and decoding accepts both.

```mermaid
flowchart LR
  C[category code] --> R{Rules}
  R -->|newcomer, R2.1| D[LICENSEDATE]
  R -->|senior or junior, R2.1| A[AGE]
  R --> M[missing: signature, license date, age]
```

## Usage

```toml
[dependencies]
rxsl = { git = "https://github.com/autodyne/rxsl" }
```

```rust
use rxsl::formats;

let adif = formats().into_iter().find(|f| f.kind() == "adis").unwrap();
let items = adif.read(bytes)?;
let text = adif.write(&items)?;

let mut sheet = summary.to_sheet();
sheet.embed(&rxsl::formats::jarl::Jarl::default(), &items)?;
let bytes = rxsl::sheets::jarl::JarlSheet.encode(&sheet)?;
let items = rxsl::sheets::unpack(&bytes)?;
```

## License

### Author

[無線部開発班](https://nextzlog.dev)

### Clauses

- This library is released under the [MIT License](LICENSE).
