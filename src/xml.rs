//! A minimal XML writer.
//!
//! # Why not a library
//!
//! This crate emits a fixed, known set of elements whose order is fixed by the
//! OOXML schema. A general XML library would add a dependency, an error path,
//! and a builder API for no benefit over writing the strings directly. What it
//! *would* buy — correct escaping — is thirty lines, below.

/// Escapes a value for XML text or an attribute.
///
/// # One pass
///
/// Each character is escaped once, in a single pass. Chained `replace` calls
/// would need `&` escaped first, or the `&` in `&#39;` would be escaped again
/// into `&amp;#39;`; a single pass cannot double-escape.
///
/// The apostrophe becomes the numeric reference `&#39;` rather than `&apos;`
/// because that is what excelize writes, and every part this crate is measured
/// against uses it.
///
/// # Characters XML 1.0 cannot carry
///
/// Control characters other than tab, newline and carriage return, and U+FFFE
/// and U+FFFF, are not legal in any XML 1.0 document, escaped or not. A stray
/// NUL pasted into a title makes Excel report the whole file damaged, so they
/// are dropped rather than written.
pub(crate) fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    escape_into(&mut out, value);
    out
}

/// [`escape`], appending to `out` rather than allocating.
pub(crate) fn escape_into(out: &mut String, value: &str) {
    for c in value.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '\'' => out.push_str("&#39;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\t'
            | '\n'
            | '\r'
            | '\u{20}'..='\u{D7FF}'
            | '\u{E000}'..='\u{FFFD}'
            | '\u{10000}'..='\u{10FFFF}' => out.push(c),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_ampersand_is_escaped_before_an_apostrophe_so_it_is_not_doubled() {
        assert_eq!(escape("'A&B'!$A$1"), "&#39;A&amp;B&#39;!$A$1");
    }

    #[test]
    fn angle_brackets_and_quotes_are_escaped() {
        assert_eq!(escape(r#"<a href="x">"#), "&lt;a href=&quot;x&quot;&gt;");
    }

    #[test]
    fn characters_xml_cannot_carry_are_dropped() {
        assert_eq!(escape("a\u{0}b\u{1B}c\u{FFFE}d"), "abcd");
        assert_eq!(escape("tab\there\nnewline"), "tab\there\nnewline");
    }

    #[test]
    fn text_needing_nothing_is_returned_unchanged() {
        assert_eq!(
            escape("Monthly totals by region"),
            "Monthly totals by region"
        );
    }
}
