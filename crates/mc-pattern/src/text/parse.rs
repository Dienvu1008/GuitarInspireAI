//! The parser of the pattern text (docs/architecture/pattern-text-syntax.md). It is
//! scannerless, because `.` and `..` mean different things in a duration and in a range.
//! It builds the IR and reports the first syntax error with a byte span, a line and a
//! column. Musical validity is the semantic stage (`PatternLibrary`).

use std::ops::Range;

use mc_theory::fretted::StringIndex;
use mc_theory::{BaseValue, NoteValue, Rational, TimeSignature, Tuplet};

use super::{ParseError, ParseErrorKind as K};
use crate::library::PatternLibrary;
use crate::model::{
    Action, Direction, Lane, LaneId, Lanes, Origin, PatternMeta, PatternParts, Step, Strike,
    StringSelector, Technique, TechniqueSet, TechniqueSpan,
};
use crate::names::{KEYWORDS, name_key};

type Res<T> = Result<T, ParseError>;

/// Marks and wrappers read before a strike's targets.
#[derive(Default)]
struct Flags {
    set: TechniqueSet,
}

struct P<'a> {
    src: &'a str,
    pos: usize,
    lib: &'a PatternLibrary,
    name: Option<String>,
    length: Option<Rational>,
    meter: Option<TimeSignature>,
    description: Option<String>,
    tags: Option<Vec<String>>,
    origin: Option<Origin>,
    unit: Option<NoteValue>,
    strum: Option<Lane>,
    thumb: Option<Lane>,
    fingers: Option<Lane>,
    spans: Vec<TechniqueSpan>,
    cur: Option<LaneId>,
    /// The position after the last step of the current lane.
    at: Rational,
    /// Bar checks, validated against the meter once the whole text is read.
    checks: Vec<(Range<usize>, Rational)>,
}

pub(super) fn parse(src: &str, lib: &PatternLibrary) -> Res<PatternParts> {
    let mut p = P {
        src,
        pos: 0,
        lib,
        name: None,
        length: None,
        meter: None,
        description: None,
        tags: None,
        origin: None,
        unit: None,
        strum: None,
        thumb: None,
        fingers: None,
        spans: Vec::new(),
        cur: None,
        at: Rational::ZERO,
        checks: Vec::new(),
    };
    p.file()?;
    p.finish()
}

fn lane_word(w: &str) -> Option<LaneId> {
    match w {
        "strum" => Some(LaneId::Strum),
        "thumb" => Some(LaneId::Thumb),
        "fingers" => Some(LaneId::Fingers),
        _ => None,
    }
}

impl<'a> P<'a> {
    // ------------------------------------------------------------ characters

    fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn peek_n(&self, n: usize) -> Option<char> {
        self.src[self.pos..].chars().nth(n)
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn err(&self, span: Range<usize>, kind: K) -> ParseError {
        let before = &self.src[..span.start];
        let line = before.matches('\n').count() + 1;
        let column = before.rsplit('\n').next().map_or(0, |l| l.chars().count()) + 1;
        ParseError {
            span,
            line,
            column,
            kind,
        }
    }

    /// The span of the current character (empty at the end).
    fn here_span(&self) -> Range<usize> {
        let end = self.peek().map_or(self.pos, |c| self.pos + c.len_utf8());
        self.pos..end
    }

    /// An error at the current character (or an empty span at the end).
    fn here(&self, kind: K) -> ParseError {
        let end = self.peek().map_or(self.pos, |c| self.pos + c.len_utf8());
        self.err(self.pos..end, kind)
    }

    fn skip_trivia(&mut self) {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => {
                    self.bump();
                }
                Some('/') if self.peek_n(1) == Some('/') => {
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.bump();
                    }
                }
                _ => break,
            }
        }
    }

    fn at_line_start(&self, pos: usize) -> bool {
        let before = &self.src[..pos];
        let line = before.rsplit('\n').next().unwrap_or("");
        line.chars().all(char::is_whitespace)
    }

    fn expect(&mut self, c: char, what: &'static str) -> Res<()> {
        if self.peek() == Some(c) {
            self.bump();
            Ok(())
        } else {
            Err(self.here(K::Expected(what)))
        }
    }

    /// The identifier at `pos`: letters, then letters and digits.
    fn ident_at(&self, pos: usize) -> Option<&'a str> {
        let src: &'a str = self.src;
        let rest = &src[pos..];
        let mut chars = rest.char_indices();
        let (_, first) = chars.next()?;
        if !first.is_alphabetic() {
            return None;
        }
        let end = chars
            .find(|(_, c)| !c.is_alphanumeric())
            .map_or(rest.len(), |(i, _)| i);
        Some(&rest[..end])
    }

    fn ident(&self) -> Option<&'a str> {
        self.ident_at(self.pos)
    }

    // --------------------------------------------------------------- numbers

    fn digits(&mut self, what: &'static str) -> Res<(Range<usize>, &'a str)> {
        let start = self.pos;
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.bump();
        }
        if self.pos == start {
            return Err(self.here(K::Expected(what)));
        }
        Ok((start..self.pos, &self.src[start..self.pos]))
    }

    fn uint(&mut self, what: &'static str) -> Res<(Range<usize>, u32)> {
        let (span, text) = self.digits(what)?;
        let n = text
            .parse::<u32>()
            .map_err(|_| self.err(span.clone(), K::BadNumber))?;
        Ok((span, n))
    }

    /// `-`? digits (`/` digits)?
    fn rational(&mut self) -> Res<Rational> {
        let start = self.pos;
        let neg = self.peek() == Some('-');
        if neg {
            self.bump();
        }
        let (_, n) = self.digits("a number")?;
        let n: i64 = n
            .parse()
            .map_err(|_| self.err(start..self.pos, K::BadRational))?;
        let d: i64 = if self.peek() == Some('/') {
            self.bump();
            let (_, d) = self.digits("a denominator")?;
            d.parse()
                .map_err(|_| self.err(start..self.pos, K::BadRational))?
        } else {
            1
        };
        Rational::new(if neg { -n } else { n }, d)
            .map_err(|_| self.err(start..self.pos, K::BadRational))
    }

    fn string(&mut self) -> Res<String> {
        let start = self.pos;
        self.expect('"', "a quoted string")?;
        let mut out = String::new();
        loop {
            let at = self.pos;
            match self.bump() {
                None => return Err(self.err(start..self.pos, K::UnterminatedString)),
                Some('"') => return Ok(out),
                Some('\\') => {
                    let c = match self.bump() {
                        Some('"') => '"',
                        Some('\\') => '\\',
                        Some('n') => '\n',
                        Some('r') => '\r',
                        Some('t') => '\t',
                        Some('u') if self.peek() == Some('{') => {
                            self.bump();
                            let h = self.pos;
                            while self.peek().is_some_and(|c| c.is_ascii_hexdigit()) {
                                self.bump();
                            }
                            let hex = &self.src[h..self.pos];
                            if self.peek() != Some('}') || hex.is_empty() || hex.len() > 6 {
                                return Err(self.err(at..self.pos, K::BadEscape));
                            }
                            self.bump();
                            u32::from_str_radix(hex, 16)
                                .ok()
                                .and_then(char::from_u32)
                                .ok_or_else(|| self.err(at..self.pos, K::BadEscape))?
                        }
                        _ => return Err(self.err(at..self.pos, K::BadEscape)),
                    };
                    out.push(c);
                }
                Some(c) => out.push(c),
            }
        }
    }

    // ------------------------------------------------------------ durations

    fn duration(&mut self) -> Res<NoteValue> {
        let start = self.pos;
        let (_, n) = self.uint("a note value")?;
        let bad = |p: &P| p.err(start..p.pos, K::BadDuration);
        if !n.is_power_of_two() || n > 256 {
            return Err(bad(self));
        }
        let base = BaseValue::from_exponent(n.trailing_zeros() as u8).map_err(|_| bad(self))?;
        let mut dots = 0u8;
        // In a duration every dot is a dot (`4..` is double dotted); a range `a..b`
        // never follows a duration.
        while self.peek() == Some('.') {
            self.bump();
            dots = dots.saturating_add(1);
        }
        let tuplet = match self.peek() {
            Some('t' | 'T') if !self.peek_n(1).is_some_and(char::is_alphanumeric) => {
                self.bump();
                Some(Tuplet::TRIPLET)
            }
            Some('(') => {
                self.bump();
                let (_, a) = self.uint("a tuplet count")?;
                self.expect(':', "':'")?;
                let (_, b) = self.uint("a tuplet count")?;
                self.expect(')', "')'")?;
                let (a, b) = (u8::try_from(a), u8::try_from(b));
                match (a, b) {
                    (Ok(a), Ok(b)) => Some(Tuplet::new(a, b).map_err(|_| bad(self))?),
                    _ => return Err(self.err(start..self.pos, K::BadTuplet)),
                }
            }
            _ => None,
        };
        NoteValue::new(base, dots, tuplet).map_err(|_| bad(self))
    }

    // ------------------------------------------------------------ the file

    fn file(&mut self) -> Res<()> {
        loop {
            self.skip_trivia();
            let Some(c) = self.peek() else { return Ok(()) };
            match c {
                '"' => self.header()?,
                c if c.is_alphabetic() => {
                    let start = self.pos;
                    let word = self.ident().unwrap_or("");
                    let lower = name_key(word);
                    if lower == "meter" {
                        self.meter()?;
                    } else if !(self.at_line_start(start)
                        && (self.lane_header(&lower)? || self.span_statement(&lower)?))
                    {
                        self.lane_item()?;
                    }
                }
                _ => self.lane_item()?,
            }
        }
    }

    fn header(&mut self) -> Res<()> {
        let start = self.pos;
        let key = self.string()?;
        let key_span = start..self.pos;
        self.skip_trivia();
        self.expect(':', "':' after the key")?;
        self.skip_trivia();
        let dup = |p: &P, name: &'static str| p.err(key_span.clone(), K::DuplicateHeader(name));
        match name_key(&key).as_str() {
            "name" => {
                if self.name.is_some() {
                    return Err(dup(self, "name"));
                }
                self.name = Some(self.string()?);
            }
            "description" => {
                if self.description.is_some() {
                    return Err(dup(self, "description"));
                }
                self.description = Some(self.string()?);
            }
            "length" => {
                if self.length.is_some() {
                    return Err(dup(self, "length"));
                }
                self.length = Some(self.rational()?);
            }
            "tags" => {
                if self.tags.is_some() {
                    return Err(dup(self, "tags"));
                }
                let mut tags = vec![self.string()?];
                loop {
                    let save = self.pos;
                    self.skip_trivia();
                    if self.peek() == Some(',') {
                        self.bump();
                        self.skip_trivia();
                        tags.push(self.string()?);
                    } else {
                        self.pos = save;
                        break;
                    }
                }
                self.tags = Some(tags);
            }
            "origin" => {
                if self.origin.is_some() {
                    return Err(dup(self, "origin"));
                }
                let at = self.pos;
                let value = self.string()?;
                self.origin = Some(match name_key(&value).as_str() {
                    "grid" => Origin::Grid,
                    "text" => Origin::Text,
                    "tap" => Origin::Tap,
                    _ => return Err(self.err(at..self.pos, K::BadOrigin(value))),
                });
            }
            _ => return Err(self.err(key_span, K::UnknownKey(key))),
        }
        Ok(())
    }

    fn meter(&mut self) -> Res<()> {
        let start = self.pos;
        self.pos += "meter".len();
        if self.meter.is_some() {
            return Err(self.err(start..self.pos, K::DuplicateHeader("meter")));
        }
        self.skip_trivia();
        let at = self.pos;
        let (_, n) = self.uint("a number")?;
        self.expect('/', "'/'")?;
        let (_, d) = self.uint("a number")?;
        let sig = match (u8::try_from(n), u8::try_from(d)) {
            (Ok(n), Ok(d)) => TimeSignature::new(n, d).ok(),
            _ => None,
        };
        self.meter = Some(sig.ok_or_else(|| self.err(at..self.pos, K::BadMeter))?);
        Ok(())
    }

    /// `strum:`, `thumb:` or `fingers:` at the start of a line, then white space or the
    /// end. Returns false (consuming nothing) if this is not a lane header.
    fn lane_header(&mut self, lower: &str) -> Res<bool> {
        let Some(lane) = lane_word(lower) else {
            return Ok(false);
        };
        let start = self.pos;
        let after = start + lower.len();
        let rest = &self.src[after..];
        let Some(tail) = rest.strip_prefix(':') else {
            return Ok(false);
        };
        if tail.chars().next().is_some_and(|c| !c.is_whitespace()) {
            return Ok(false); // `Strum:4` is a step
        }
        self.pos = after + 1;
        let span = start..self.pos;
        let mix = matches!(
            (lane, &self.strum, &self.thumb, &self.fingers),
            (LaneId::Strum, _, Some(_), _)
                | (LaneId::Strum, _, _, Some(_))
                | (LaneId::Thumb | LaneId::Fingers, Some(_), _, _)
        );
        if mix {
            return Err(self.err(span, K::LaneMix));
        }
        let slot = match lane {
            LaneId::Strum => &mut self.strum,
            LaneId::Thumb => &mut self.thumb,
            LaneId::Fingers => &mut self.fingers,
        };
        if slot.is_some() {
            return Err(self.err(span, K::DuplicateLane));
        }
        *slot = Some(Lane::default());
        self.cur = Some(lane);
        self.at = Rational::ZERO;
        Ok(true)
    }

    fn technique_token(&mut self) -> Option<Technique> {
        match self.peek()? {
            '>' => {
                self.bump();
                Some(Technique::Accent)
            }
            '!' => {
                self.bump();
                Some(Technique::Dead)
            }
            '~' => {
                self.bump();
                Some(Technique::LetRing)
            }
            c if c.is_alphabetic() => {
                let word = name_key(self.ident()?);
                let t = match word.as_str() {
                    "pm" => Technique::PalmMute,
                    "sp" => Technique::StringPercussive,
                    _ => return None,
                };
                self.pos += word.len();
                Some(t)
            }
            _ => None,
        }
    }

    /// `<lane> <technique>#a` or `#a..b` at the start of a line. Returns false
    /// (consuming nothing) if the line does not have that shape.
    fn span_statement(&mut self, lower: &str) -> Res<bool> {
        let Some(lane) = lane_word(lower) else {
            return Ok(false);
        };
        let save = self.pos;
        self.pos += lower.len();
        while matches!(self.peek(), Some(' ' | '\t')) {
            self.bump();
        }
        let Some(technique) = self.technique_token() else {
            self.pos = save;
            return Ok(false);
        };
        if self.peek() != Some('#') {
            self.pos = save;
            return Ok(false);
        }
        self.bump();
        let (_, first) = self.uint("a step index")?;
        let last = if self.peek() == Some('.') && self.peek_n(1) == Some('.') {
            self.bump();
            self.bump();
            self.uint("a step index")?.1
        } else {
            first
        };
        self.spans.push(TechniqueSpan {
            lane,
            first: first as usize,
            last: last as usize,
            technique,
        });
        Ok(true)
    }

    // ------------------------------------------------------------ lane items

    fn lane_slot(&mut self, at: Range<usize>) -> Res<&mut Lane> {
        let Some(lane) = self.cur else {
            return Err(self.err(at, K::StepOutsideLane));
        };
        Ok(match lane {
            LaneId::Strum => self.strum.as_mut(),
            LaneId::Thumb => self.thumb.as_mut(),
            LaneId::Fingers => self.fingers.as_mut(),
        }
        .expect("the current lane exists"))
    }

    /// One item of a lane: a bar check, a unit, a bracketed span, or a step.
    fn lane_item(&mut self) -> Res<()> {
        let start = self.pos;
        match self.peek() {
            Some('|') => {
                self.bump();
                self.lane_slot(start..self.pos)?;
                self.checks.push((start..self.pos, self.at));
                Ok(())
            }
            Some('[') => self.bracket(),
            Some(c) if c.is_alphabetic() && self.ident().is_some_and(|w| name_key(w) == "unit") => {
                self.pos += "unit".len();
                self.skip_trivia();
                self.unit = Some(self.duration()?);
                Ok(())
            }
            _ => self.step(),
        }
    }

    fn bracket(&mut self) -> Res<()> {
        let open = self.pos;
        self.bump();
        self.lane_slot(open..self.pos)?;
        self.skip_trivia();
        let Some(technique) = self.technique_token() else {
            return Err(self.here(K::Expected("a technique after '['")));
        };
        let lane = self.cur.expect("checked");
        let first = self.lane_slot(open..open + 1)?.steps.len();
        loop {
            self.skip_trivia();
            match self.peek() {
                Some(']') => {
                    self.bump();
                    break;
                }
                None => return Err(self.err(open..open + 1, K::UnclosedBracket)),
                _ => self.lane_item()?,
            }
        }
        let n = self.lane_slot(open..open + 1)?.steps.len();
        if n == first {
            return Err(self.err(open..self.pos, K::EmptyBracket));
        }
        self.spans.push(TechniqueSpan {
            lane,
            first,
            last: n - 1,
            technique,
        });
        Ok(())
    }

    fn step(&mut self) -> Res<()> {
        let start = self.pos;
        self.lane_slot(self.here_span())?;
        let mut onset = None;
        if self.peek() == Some('@') {
            self.bump();
            onset = Some(self.rational()?);
            self.skip_trivia();
        }
        let head = self.pos; // direction, marks and action: the span of a value error
        let mut direction = None;
        if let Some(w) = self.ident() {
            let lower = name_key(w);
            if lower == "d" || lower == "u" {
                direction = Some(if lower == "d" {
                    Direction::Down
                } else {
                    Direction::Up
                });
                self.pos += w.len();
                self.skip_trivia();
            }
        }
        let mut flags = Flags::default();
        let mut marked = false;
        while let Some(c) = self.peek() {
            match c {
                '>' => flags.set.accent = true,
                '!' => flags.set.dead = true,
                '~' => flags.set.let_ring = true,
                _ => break,
            }
            marked = true;
            self.bump();
        }
        let action = self.action(&mut flags)?;
        let plain = !matches!(action, Parsed::Strike(_));
        if plain && (direction.is_some() || marked) {
            return Err(self.err(head..self.pos, K::MarksOnNonStrike));
        }
        let duration = if self.peek() == Some(':') {
            self.bump();
            self.duration()?
        } else {
            self.unit
                .ok_or_else(|| self.err(head..self.pos, K::NoValue))?
        };
        let action = match action {
            Parsed::Strike(targets) => Action::Strike(Strike {
                targets,
                direction,
                techniques: flags.set,
            }),
            Parsed::Gesture(id) => Action::Gesture(id),
            Parsed::Hold(sel) => Action::Hold(sel),
            Parsed::Rest => Action::Rest,
        };
        let onset = onset.unwrap_or(self.at);
        let end = onset
            .checked_add(duration.length())
            .ok_or_else(|| self.err(start..self.pos, K::TimeOverflow))?;
        self.at = end;
        let step = Step {
            onset,
            duration,
            action,
        };
        self.lane_slot(start..self.pos)?.steps.push(step);
        Ok(())
    }

    fn action(&mut self, flags: &mut Flags) -> Res<Parsed> {
        match self.peek() {
            Some('-') => {
                self.bump();
                Ok(Parsed::Rest)
            }
            Some('_') => {
                self.bump();
                if self.peek() == Some('(') {
                    self.bump();
                    let sel = self.selectors()?;
                    self.expect(')', "')'")?;
                    Ok(Parsed::Hold(Some(sel)))
                } else {
                    Ok(Parsed::Hold(None))
                }
            }
            Some(c) if c.is_ascii_digit() => Ok(Parsed::Strike(self.selectors()?)),
            Some(c) if c.is_alphabetic() => {
                let start = self.pos;
                let word = self.ident().unwrap_or("");
                let lower = name_key(word);
                let is_wrapper = (lower == "pm" || lower == "sp")
                    && self.peek_n(word.chars().count()) == Some('(');
                if is_wrapper || matches!(lower.as_str(), "bass" | "alt" | "treble" | "strum") {
                    return Ok(Parsed::Strike(self.strike_body(flags)?));
                }
                if KEYWORDS.contains(&lower.as_str()) {
                    return Err(
                        self.err(start..start + word.len(), K::KeywordNotAllowed(word.into()))
                    );
                }
                match self.lib.gesture_by_name(word) {
                    Some(g) => {
                        self.pos += word.len();
                        Ok(Parsed::Gesture(g.id()))
                    }
                    None => {
                        Err(self.err(start..start + word.len(), K::UnknownGesture(word.into())))
                    }
                }
            }
            _ => Err(self.here(K::Expected("a step"))),
        }
    }

    /// `pm(...)` and `sp(...)` wrappers (nestable) around a `+` list of selectors.
    fn strike_body(&mut self, flags: &mut Flags) -> Res<Vec<StringSelector>> {
        if let Some(w) = self.ident() {
            let lower = name_key(w);
            let wrapper = match lower.as_str() {
                "pm" => Some(Technique::PalmMute),
                "sp" => Some(Technique::StringPercussive),
                _ => None,
            };
            if let Some(t) = wrapper
                && self.peek_n(w.chars().count()) == Some('(')
            {
                self.pos += w.len() + 1;
                match t {
                    Technique::PalmMute => flags.set.palm_mute = true,
                    _ => flags.set.string_percussive = true,
                }
                self.skip_trivia();
                let inner = self.strike_body(flags)?;
                self.skip_trivia();
                self.expect(')', "')'")?;
                return Ok(inner);
            }
        }
        self.selectors()
    }

    fn selectors(&mut self) -> Res<Vec<StringSelector>> {
        let mut out = vec![self.selector()?];
        while self.peek() == Some('+') {
            self.bump();
            out.push(self.selector()?);
        }
        Ok(out)
    }

    fn string_index(&mut self, span: Range<usize>, n: u32) -> Res<StringIndex> {
        u8::try_from(n)
            .ok()
            .and_then(|n| StringIndex::new(n).ok())
            .ok_or_else(|| self.err(span, K::InvalidString(n)))
    }

    fn selector(&mut self) -> Res<StringSelector> {
        let start = self.pos;
        if self.peek().is_some_and(|c| c.is_ascii_digit()) {
            let (span, n) = self.uint("a string number")?;
            return Ok(StringSelector::Absolute(self.string_index(span, n)?));
        }
        let word = self
            .ident()
            .ok_or_else(|| self.here(K::Expected("a string selector")))?;
        let lower = name_key(word);
        self.pos += word.len();
        match lower.as_str() {
            "bass" => Ok(StringSelector::Bass),
            "alt" => Ok(StringSelector::Alt),
            "treble" => {
                if self.peek() == Some('(') {
                    self.bump();
                    let (span, n) = self.uint("a string count")?;
                    self.expect(')', "')'")?;
                    let n = u8::try_from(n).map_err(|_| self.err(span, K::BadNumber))?;
                    Ok(StringSelector::Treble(n))
                } else {
                    Ok(StringSelector::TREBLE)
                }
            }
            "strum" => {
                if self.peek() == Some('(') {
                    self.bump();
                    let (sa, a) = self.uint("a string number")?;
                    if !(self.peek() == Some('.') && self.peek_n(1) == Some('.')) {
                        return Err(self.here(K::Expected("'..'")));
                    }
                    self.bump();
                    self.bump();
                    let (sb, b) = self.uint("a string number")?;
                    self.expect(')', "')'")?;
                    Ok(StringSelector::StrumRange(
                        self.string_index(sa, a)?,
                        self.string_index(sb, b)?,
                    ))
                } else {
                    Ok(StringSelector::Strum)
                }
            }
            _ => Err(self.err(start..self.pos, K::Expected("a string selector"))),
        }
    }

    // --------------------------------------------------------------- the end

    fn finish(self) -> Res<PatternParts> {
        let end = self.src.len()..self.src.len();
        let length = self
            .length
            .ok_or_else(|| self.err(end.clone(), K::MissingLength))?;
        for (span, pos) in &self.checks {
            let Some(meter) = self.meter else {
                return Err(self.err(span.clone(), K::BarCheckWithoutMeter));
            };
            let bar = meter.length();
            let on_line = pos.checked_div(bar).is_some_and(|q| q.denominator() == 1);
            if !on_line {
                return Err(self.err(
                    span.clone(),
                    K::BarCheckOffBarLine {
                        position: *pos,
                        bar_length: bar,
                    },
                ));
            }
        }
        let lanes = match (self.strum.clone(), self.thumb.clone(), self.fingers.clone()) {
            (Some(l), None, None) => Lanes::Strum(l),
            (None, Some(thumb), Some(fingers)) => Lanes::ThumbFingers { thumb, fingers },
            (None, Some(_), None) => return Err(self.err(end, K::MissingFingers)),
            (None, None, Some(_)) => return Err(self.err(end, K::MissingThumb)),
            (None, None, None) => return Err(self.err(end, K::NoLane)),
            _ => return Err(self.err(end, K::LaneMix)),
        };
        Ok(PatternParts {
            name: self.name.unwrap_or_default(),
            length,
            meter: self.meter,
            lanes,
            spans: self.spans,
            meta: PatternMeta {
                description: self.description.unwrap_or_default(),
                tags: self.tags.unwrap_or_default(),
                origin: self.origin,
            },
        })
    }
}

enum Parsed {
    Strike(Vec<StringSelector>),
    Gesture(crate::ids::GestureId),
    Hold(Option<Vec<StringSelector>>),
    Rest,
}
