//! A minimal XML writer: nesting, two-space indentation, escaping. Text and attribute
//! values drop the characters XML 1.0 forbids and count them.

#[derive(Debug, Default)]
pub(crate) struct Xml {
    out: String,
    stack: Vec<&'static str>,
    /// Characters removed because XML 1.0 does not allow them.
    pub(crate) removed: usize,
}

/// XML 1.0 `Char`: tab, newline, carriage return, and U+0020 upward except the two
/// non-characters U+FFFE and U+FFFF (Rust strings hold no surrogates).
fn allowed(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\r') || (c >= ' ' && c != '\u{FFFE}' && c != '\u{FFFF}')
}

impl Xml {
    pub(crate) fn new() -> Self {
        let mut x = Xml::default();
        x.out
            .push_str("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?>\n");
        x
    }

    fn indent(&mut self) {
        for _ in 0..self.stack.len() {
            self.out.push_str("  ");
        }
    }

    fn escaped(&mut self, s: &str, attribute: bool) {
        for c in s.chars() {
            if !allowed(c) {
                self.removed += 1;
                continue;
            }
            match c {
                '&' => self.out.push_str("&amp;"),
                '<' => self.out.push_str("&lt;"),
                '>' => self.out.push_str("&gt;"),
                '"' if attribute => self.out.push_str("&quot;"),
                '\'' if attribute => self.out.push_str("&apos;"),
                // Keep line breaks and tabs in attributes as character references so a
                // reader's attribute normalisation does not turn them into spaces.
                '\n' if attribute => self.out.push_str("&#10;"),
                '\r' => self.out.push_str("&#13;"),
                '\t' if attribute => self.out.push_str("&#9;"),
                c => self.out.push(c),
            }
        }
    }

    fn start_tag(&mut self, name: &str, attrs: &[(&str, &str)]) {
        self.indent();
        self.out.push('<');
        self.out.push_str(name);
        for (k, v) in attrs {
            self.out.push(' ');
            self.out.push_str(k);
            self.out.push_str("=\"");
            self.escaped(v, true);
            self.out.push('"');
        }
    }

    /// `<name attrs>` and descend.
    pub(crate) fn open(&mut self, name: &'static str, attrs: &[(&str, &str)]) {
        self.start_tag(name, attrs);
        self.out.push_str(">\n");
        self.stack.push(name);
    }

    /// Close the innermost open element.
    pub(crate) fn close(&mut self) {
        let name = self.stack.pop().expect("balanced");
        self.indent();
        self.out.push_str("</");
        self.out.push_str(name);
        self.out.push_str(">\n");
    }

    /// `<name attrs/>`.
    pub(crate) fn empty(&mut self, name: &str, attrs: &[(&str, &str)]) {
        self.start_tag(name, attrs);
        self.out.push_str("/>\n");
    }

    /// `<name attrs>text</name>`.
    pub(crate) fn text(&mut self, name: &str, attrs: &[(&str, &str)], text: &str) {
        self.start_tag(name, attrs);
        self.out.push('>');
        self.escaped(text, false);
        self.out.push_str("</");
        self.out.push_str(name);
        self.out.push_str(">\n");
    }

    pub(crate) fn finish(self) -> (String, usize) {
        debug_assert!(self.stack.is_empty());
        (self.out, self.removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_and_removes_forbidden_characters() {
        let mut x = Xml::new();
        x.open("a", &[("v", "\"x' & <y>\n")]);
        x.text("b", &[], "1 < 2 & \u{0}\u{1F} ok\u{FFFF}");
        x.close();
        let (s, removed) = x.finish();
        assert!(s.contains("v=\"&quot;x&apos; &amp; &lt;y&gt;&#10;\""));
        assert!(s.contains("<b>1 &lt; 2 &amp;  ok</b>"));
        assert_eq!(removed, 3);
    }
}
