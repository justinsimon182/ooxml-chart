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
/// # Why the order matters
///
/// `&` is escaped **first**. Escaping the apostrophe first would turn `'` into
/// `&#39;` and then the `&` pass would turn that into `&amp;#39;`, which is the
/// literal text `&#39;` rather than an apostrophe.
///
/// The apostrophe becomes the numeric reference `&#39;` rather than `&apos;`
/// because that is what excelize writes, and every part this crate is measured
/// against uses it.
pub fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('\'', "&#39;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
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
    fn text_needing_nothing_is_returned_unchanged() {
        assert_eq!(
            escape("Monthly totals by region"),
            "Monthly totals by region"
        );
    }
}
