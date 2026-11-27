//! Test support: a small reader for the PDFs `mc-pdf` writes. It reads objects,
//! Flate streams, the page tree, the font resources, the ToUnicode maps, and the
//! operators the painter uses (cm, w, G, g, m, l, c, h, S, f, BT, Tf, Tm, Tj, ET).
//! Not a general PDF parser.

use std::collections::BTreeMap;

pub struct Obj {
    pub dict: String,
    pub stream: Option<Vec<u8>>,
}

pub struct Doc {
    pub objs: BTreeMap<u32, Obj>,
}

fn find(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    hay[from..]
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|i| i + from)
}

impl Doc {
    pub fn parse(pdf: &[u8]) -> Doc {
        let mut objs = BTreeMap::new();
        let mut at = 0;
        while let Some(i) = find(pdf, b" 0 obj\n", at) {
            let start = pdf[..i]
                .iter()
                .rposition(|b| *b == b'\n')
                .map_or(0, |p| p + 1);
            let num: u32 = std::str::from_utf8(&pdf[start..i])
                .unwrap()
                .parse()
                .unwrap();
            let body = i + 7;
            let end = find(pdf, b"endobj", body).expect("endobj");
            let (dict, stream) = match find(&pdf[..end], b"stream\n", body) {
                Some(s) => {
                    let dict = String::from_utf8_lossy(&pdf[body..s]).to_string();
                    let len = int_after(&dict, "/Length ").expect("stream length") as usize;
                    (dict, Some(pdf[s + 7..s + 7 + len].to_vec()))
                }
                None => (String::from_utf8_lossy(&pdf[body..end]).to_string(), None),
            };
            objs.insert(num, Obj { dict, stream });
            at = end;
        }
        Doc { objs }
    }

    pub fn obj(&self, n: u32) -> &Obj {
        &self.objs[&n]
    }

    /// A stream's bytes, inflated when Flate-encoded.
    pub fn data(&self, n: u32) -> Vec<u8> {
        let o = self.obj(n);
        let raw = o.stream.as_ref().expect("a stream");
        if o.dict.contains("/FlateDecode") {
            miniz_oxide::inflate::decompress_to_vec_zlib(raw).expect("inflates")
        } else {
            raw.clone()
        }
    }

    /// Page objects in page order.
    pub fn pages(&self) -> Vec<u32> {
        let tree = self
            .objs
            .values()
            .find(|o| o.dict.contains("/Type /Pages"))
            .expect("page tree");
        refs(between(&tree.dict, "/Kids [", "]"))
    }

    pub fn media_box(&self, page: u32) -> Vec<f32> {
        between(&self.obj(page).dict, "/MediaBox [", "]")
            .split_whitespace()
            .map(|v| v.parse().unwrap())
            .collect()
    }

    /// The font resources of a page: resource name to Type 0 font object.
    pub fn page_fonts(&self, page: u32) -> BTreeMap<String, u32> {
        let d = &self.obj(page).dict;
        let fonts = between(d, "/Font <<", ">>");
        let mut out = BTreeMap::new();
        let toks: Vec<&str> = fonts.split_whitespace().collect();
        for w in toks.windows(4) {
            if w[0].starts_with('/') && w[3] == "R" {
                out.insert(w[0][1..].to_string(), w[1].parse().unwrap());
            }
        }
        out
    }

    pub fn content(&self, page: u32) -> Content {
        let n = int_after(&self.obj(page).dict, "/Contents ").expect("contents") as u32;
        Content::parse(&self.data(n))
    }

    /// CID to character, from a Type 0 font's ToUnicode map.
    pub fn to_unicode(&self, font: u32) -> BTreeMap<u16, char> {
        let n = int_after(&self.obj(font).dict, "/ToUnicode ").expect("ToUnicode") as u32;
        let text = String::from_utf8(self.data(n)).unwrap();
        let mut out = BTreeMap::new();
        for section in text.split("beginbfchar").skip(1) {
            let section = section.split("endbfchar").next().unwrap();
            let hex: Vec<&str> = section
                .split(['<', '>'])
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .collect();
            for pair in hex.chunks(2) {
                let cid = u16::from_str_radix(pair[0], 16).unwrap();
                let units: Vec<u16> = (0..pair[1].len() / 4)
                    .map(|i| u16::from_str_radix(&pair[1][i * 4..i * 4 + 4], 16).unwrap())
                    .collect();
                let ch = char::decode_utf16(units).next().unwrap().unwrap();
                out.insert(cid, ch);
            }
        }
        out
    }
}

pub fn between<'a>(s: &'a str, open: &str, close: &str) -> &'a str {
    let i = s.find(open).unwrap_or_else(|| panic!("no {open} in {s}")) + open.len();
    let j = s[i..].find(close).expect("close") + i;
    &s[i..j]
}

pub fn int_after(s: &str, key: &str) -> Option<i64> {
    let i = s.find(key)? + key.len();
    s[i..]
        .split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()
}

pub fn refs(s: &str) -> Vec<u32> {
    let toks: Vec<&str> = s.split_whitespace().collect();
    toks.chunks(3).map(|c| c[0].parse().unwrap()).collect()
}

#[derive(Debug, Clone)]
pub struct Text {
    pub font: String,
    pub size: f32,
    pub x: f32,
    pub y: f32,
    pub cids: Vec<u16>,
    pub gray: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct Stroke {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub width: f32,
    pub gray: f32,
}

#[derive(Debug, Clone)]
pub struct Fill {
    /// Operator and operands, in order.
    pub path: Vec<(char, Vec<f32>)>,
    pub gray: f32,
}

#[derive(Debug, Default)]
pub struct Content {
    /// The first `cm` operands.
    pub cm: Vec<f32>,
    pub texts: Vec<Text>,
    pub strokes: Vec<Stroke>,
    pub fills: Vec<Fill>,
}

enum Tok {
    Num(f32),
    Name(String),
    Str(Vec<u8>),
    Arr(Vec<f32>),
    Op(String),
}

fn tokens(b: &[u8]) -> Vec<Tok> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c.is_ascii_whitespace() {
            i += 1;
        } else if c == b'/' {
            let s = i + 1;
            i = s;
            while i < b.len() && !b[i].is_ascii_whitespace() && !b"/[]()<>".contains(&b[i]) {
                i += 1;
            }
            out.push(Tok::Name(String::from_utf8_lossy(&b[s..i]).to_string()));
        } else if c == b'(' {
            let mut v = Vec::new();
            let mut depth = 1;
            i += 1;
            while depth > 0 {
                match b[i] {
                    b'\\' => {
                        i += 1;
                        match b[i] {
                            b'n' => v.push(b'\n'),
                            b'r' => v.push(b'\r'),
                            b't' => v.push(b'\t'),
                            b'b' => v.push(8),
                            b'f' => v.push(12),
                            d @ b'0'..=b'7' => {
                                let mut n = u32::from(d - b'0');
                                for _ in 0..2 {
                                    if (b'0'..=b'7').contains(&b[i + 1]) {
                                        i += 1;
                                        n = n * 8 + u32::from(b[i] - b'0');
                                    }
                                }
                                v.push(n as u8);
                            }
                            other => v.push(other),
                        }
                    }
                    b'(' => {
                        depth += 1;
                        v.push(b'(');
                    }
                    b')' => {
                        depth -= 1;
                        if depth > 0 {
                            v.push(b')');
                        }
                    }
                    other => v.push(other),
                }
                i += 1;
            }
            out.push(Tok::Str(v));
        } else if c == b'<' {
            let e = find(b, b">", i).unwrap();
            let hex: Vec<u8> = b[i + 1..e]
                .iter()
                .copied()
                .filter(|x| !x.is_ascii_whitespace())
                .collect();
            let v = hex
                .chunks(2)
                .map(|h| u8::from_str_radix(std::str::from_utf8(h).unwrap(), 16).unwrap())
                .collect();
            out.push(Tok::Str(v));
            i = e + 1;
        } else if c == b'[' {
            let e = find(b, b"]", i).unwrap();
            let nums = String::from_utf8_lossy(&b[i + 1..e])
                .split_whitespace()
                .map(|v| v.parse().unwrap())
                .collect();
            out.push(Tok::Arr(nums));
            i = e + 1;
        } else {
            let s = i;
            while i < b.len() && !b[i].is_ascii_whitespace() && !b"/[]()<>".contains(&b[i]) {
                i += 1;
            }
            let w = String::from_utf8_lossy(&b[s..i]).to_string();
            match w.parse::<f32>() {
                Ok(n) => out.push(Tok::Num(n)),
                Err(_) => out.push(Tok::Op(w)),
            }
        }
    }
    out
}

impl Content {
    pub fn parse(b: &[u8]) -> Content {
        let mut c = Content::default();
        let mut nums: Vec<f32> = Vec::new();
        let (mut width, mut stroke_gray, mut fill_gray) = (1.0, 0.0, 0.0);
        let mut path: Vec<(char, Vec<f32>)> = Vec::new();
        let (mut font, mut size, mut tm) = (String::new(), 0.0, [0.0f32; 6]);
        let mut last_name = String::new();
        let mut last_str = Vec::new();
        let mut last_arr = Vec::new();
        for t in tokens(b) {
            match t {
                Tok::Num(n) => nums.push(n),
                Tok::Name(n) => last_name = n,
                Tok::Str(s) => last_str = s,
                Tok::Arr(a) => last_arr = a,
                Tok::Op(op) => {
                    match op.as_str() {
                        "cm" if c.cm.is_empty() => c.cm = nums.clone(),
                        "w" => width = nums[0],
                        "G" => stroke_gray = nums[0],
                        "g" => fill_gray = nums[0],
                        "m" => path.push(('M', nums.clone())),
                        "l" => path.push(('L', nums.clone())),
                        "c" => path.push(('C', nums.clone())),
                        "h" => path.push(('Z', Vec::new())),
                        "S" => {
                            assert_eq!(path.len(), 2, "a stroke is one segment");
                            let (a, b) = (&path[0].1, &path[1].1);
                            c.strokes.push(Stroke {
                                x1: a[0],
                                y1: a[1],
                                x2: b[0],
                                y2: b[1],
                                width,
                                gray: stroke_gray,
                            });
                            path.clear();
                        }
                        "f" => c.fills.push(Fill {
                            path: std::mem::take(&mut path),
                            gray: fill_gray,
                        }),
                        "Tf" => {
                            font = last_name.clone();
                            size = nums[0];
                        }
                        "Tm" => tm.copy_from_slice(&nums[..6]),
                        "Tj" => {
                            assert_eq!(size, 1.0, "the size is in the text matrix");
                            assert_eq!(tm[0], -tm[3], "flipped back once");
                            c.texts.push(Text {
                                font: font.clone(),
                                size: tm[0],
                                x: tm[4],
                                y: tm[5],
                                cids: last_str
                                    .chunks(2)
                                    .map(|p| u16::from_be_bytes([p[0], p[1]]))
                                    .collect(),
                                gray: fill_gray,
                            });
                        }
                        "BT" | "ET" => {}
                        other => panic!("unexpected operator {other}"),
                    }
                    nums.clear();
                    let _ = &last_arr;
                }
            }
        }
        c
    }
}
