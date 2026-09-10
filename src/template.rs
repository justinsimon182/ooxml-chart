//! Adopting an existing chart's formatting and changing only what must change.
//!
//! # Why cloning beats building
//!
//! A chart part carries far more than its data: colours, fonts, number formats,
//! axis scaling, legend placement, gap widths, effects. When you are adding one
//! chart to a family of charts that already exist and already render correctly,
//! every one of those is a decision you can inherit for free and get wrong for
//! free. So this replaces the data references and, optionally, the title, and
//! copies every other byte across untouched.
//!
//! # Two spellings of the same element
//!
//! Excel binds the chart namespace to the prefix `c:` and writes
//! `<c:chartSpace>`, `<c:title>`, `<c:f>`. excelize binds it as the document's
//! *default* namespace and writes `<chartSpace xmlns="…/chart">`, `<title>`,
//! `<f>` — the same elements, no prefix. A workbook written by excelize is the
//! second kind, and a workbook resaved by Excel is the first, so both spellings
//! appear in real corpora and both are recognised.
//!
//! The prefix is decided **once**, from the root element ([`chart_prefix`]),
//! and every structural needle is built from it. A part that spells its
//! chart-namespace elements *both* ways — which no writer produces — is
//! refused with [`ChartError::MixedNamespacePrefixes`] rather than read under
//! one of several equally valid guesses. Deciding per element would be one
//! such guess; so is carrying on with a needle the document does not use,
//! which is worse, because a bound that finds nothing is skipped silently and
//! an axis title is then indistinguishable from the chart's. References are
//! the deliberate exception: they are matched in either spelling wherever they
//! occur, because a count mismatch there is caught by
//! [`ChartError::ReferenceCount`] rather than being guessed at.
//!
//! # Refuse rather than mangle
//!
//! This module has no XML parser and never will: everything here is byte
//! scanning. Where that is not enough to know the right answer — an edit that
//! would overlap another, a comment or CDATA section that never closes, a
//! reference element with no matching close tag — the template is refused
//! with a [`ChartError`] instead of guessed at. A chart that silently plots
//! the wrong range still opens fine and looks entirely normal; that is a
//! worse outcome than an error the caller can see.

use crate::error::ChartError;
use crate::spec::ChartPart;
use crate::xml::escape;

/// A byte range in the template to be replaced with different text.
///
/// For a reference element this is the span of its *contents* (the tags
/// themselves are left alone). For a title run being deleted outright, it is
/// the span of the *whole element*, tags included.
#[derive(Debug, Clone, Copy)]
struct Span {
    start: usize,
    end: usize,
}

/// Where the chart's own title lives, once found and bounded to a single
/// `<c:title>…</c:title>` element that precedes `<c:plotArea>`.
#[derive(Debug, Clone)]
struct TitleSpans {
    /// The content span of the first run's `<a:t>` — what `with_title`'s text
    /// replaces.
    text: Span,
    /// Every run after the first, spans covering the whole `<a:r>…</a:r>`
    /// element. Deleted outright when the title is replaced, so a
    /// multi-run title collapses to one run rather than reading as the new
    /// text with the old runs' text trailing after it.
    extra_runs: Vec<Span>,
}

/// A chart part being reused as a template.
#[derive(Debug, Clone)]
pub struct TemplateChart {
    part: Vec<u8>,
    references: Vec<Span>,
    title: Option<TitleSpans>,
    replacements: Vec<String>,
    new_title: Option<String>,
}

impl TemplateChart {
    /// Parses `part` and locates its reference and title elements.
    ///
    /// # Errors
    ///
    /// [`ChartError::NotAChart`] if the bytes hold neither a `<c:chartSpace`
    /// nor a `<chartSpace` element.
    ///
    /// [`ChartError::MixedNamespacePrefixes`] if `part` spells its
    /// chart-namespace elements both ways — a `c:`-prefixed root over a bare
    /// `<plotArea>`, or a bare root over a `<c:plotArea>`. The prefix comes
    /// from the root element alone, so a needle built from it would silently
    /// find nothing in the other spelling and skip the bound that keeps an
    /// axis title from being mistaken for the chart's.
    ///
    /// [`ChartError::UnterminatedComment`] or [`ChartError::UnterminatedCData`]
    /// if a comment or CDATA section in `part` is never closed, and
    /// [`ChartError::UnclosedReference`] if a `<c:f>` or `<f>` element is
    /// opened but never closed. All three are found while scanning `part` for
    /// its reference elements, so they surface here rather than later at
    /// [`TemplateChart::render`].
    pub(crate) fn parse(part: &[u8]) -> Result<Self, ChartError> {
        let prefix = chart_prefix(part).ok_or(ChartError::NotAChart)?;
        if mixes_namespace_prefixes(part, prefix) {
            return Err(ChartError::MixedNamespacePrefixes);
        }
        Ok(Self {
            references: reference_spans(part)?,
            title: title_spans(part, prefix),
            part: part.to_vec(),
            replacements: Vec::new(),
            new_title: None,
        })
    }

    /// How many live reference elements the byte scanner found in the
    /// template. [`TemplateChart::with_references`] must be given exactly
    /// this many.
    ///
    /// This reports what the scanner found, not independently verified
    /// ground truth: a self-closing `<c:f/>` holds no reference and is not
    /// counted, and anything inside a comment or CDATA section is skipped
    /// rather than counted. So this check verifies the caller's count
    /// against the scan, not against what the template "should" hold.
    #[must_use]
    pub fn reference_count(&self) -> usize {
        self.references.len()
    }

    /// The references to write, in document order.
    #[must_use]
    pub fn with_references(mut self, references: Vec<String>) -> Self {
        self.replacements = references;
        self
    }

    /// Replaces the template's title text.
    ///
    /// Only the chart's own title is eligible — an axis title is never
    /// touched. This is a structural guarantee, not a heuristic: axis titles
    /// live inside `<c:plotArea>`, which always follows the chart title, so
    /// a `<c:title>` found at or after the first `<c:plotArea>` is never
    /// treated as the chart's title.
    ///
    /// If the template's title holds more than one text run (Excel splits a
    /// title into runs whenever formatting differs partway through it), every
    /// run after the first is deleted when the title is replaced, and the
    /// first run's text becomes the new title — so the result reads as one
    /// title, not the new text with the old runs' text trailing after it.
    ///
    /// # Errors deferred to `render`
    ///
    /// Calling this on a template with no chart title, or one whose chart
    /// title has no plain-text run to replace (e.g. bound to a cell via
    /// `<c:strRef>`), is not an error yet — it becomes
    /// [`ChartError::NoTitleInTemplate`] when [`TemplateChart::render`] is
    /// called.
    #[must_use]
    pub fn with_title(mut self, text: impl Into<String>) -> Self {
        self.new_title = Some(text.into());
        self
    }

    /// Emits the chart part.
    ///
    /// # Errors
    ///
    /// [`ChartError::ReferenceCount`] if the number of references supplied is
    /// not the number the template holds — a chart left plotting the template's
    /// ranges looks entirely normal and is wrong.
    ///
    /// [`ChartError::NoTitleInTemplate`] if [`TemplateChart::with_title`] was
    /// used on a template with no eligible chart title.
    ///
    /// [`ChartError::OverlappingEdits`] if two of this render's edits overlap
    /// — most often a `<c:f>` reference nested inside the title text, which
    /// cannot be applied as two independent substitutions without corrupting
    /// one of them.
    pub fn render(&self) -> Result<ChartPart, ChartError> {
        if self.replacements.len() != self.references.len() {
            return Err(ChartError::ReferenceCount {
                template: self.references.len(),
                supplied: self.replacements.len(),
            });
        }
        if self.new_title.is_some() && self.title.is_none() {
            return Err(ChartError::NoTitleInTemplate);
        }

        // Edits are applied left to right over the original bytes, so the spans
        // stay valid: nothing is inserted before a span that has not been
        // copied yet.
        let mut edits: Vec<(Span, String)> = self
            .references
            .iter()
            .zip(&self.replacements)
            .map(|(span, text)| (*span, escape(text)))
            .collect();
        if let (Some(spans), Some(text)) = (&self.title, self.new_title.as_ref()) {
            edits.push((spans.text, escape(text)));
            // Every run after the first is deleted outright, not just its
            // text: leaving the `<a:r>` tags behind would still be a second,
            // empty run, and Excel is not obliged to render that gracefully.
            for run in &spans.extra_runs {
                edits.push((*run, String::new()));
            }
        }
        edits.sort_by_key(|(span, _)| span.start);

        // Two edits overlapping means applying both would corrupt one of
        // them — most commonly a reference nested inside the title text.
        // Detected before any bytes are copied, rather than trusted to a
        // slice that would otherwise panic.
        let mut copied_to = 0usize;
        for (span, _) in &edits {
            if span.start < copied_to {
                return Err(ChartError::OverlappingEdits {
                    at: span.start,
                    previous_end: copied_to,
                });
            }
            copied_to = span.end;
        }

        let mut out = Vec::with_capacity(self.part.len());
        let mut cursor = 0usize;
        for (span, text) in edits {
            out.extend_from_slice(&self.part[cursor..span.start]);
            out.extend_from_slice(text.as_bytes());
            cursor = span.end;
        }
        out.extend_from_slice(&self.part[cursor..]);

        Ok(ChartPart {
            xml: out,
            content_type: ChartPart::CONTENT_TYPE,
        })
    }
}

/// The content spans of every live `<c:f>` or `<f>` element, in document
/// order.
///
/// "Live" excludes three things a literal-needle search would not: an
/// element inside a `<!--` comment or a `<![CDATA[` section (skipped, not
/// counted — it is not the part Excel or excelize would ever read as a
/// reference), a self-closing element like `<c:f/>` (skipped — it holds no
/// content to replace), and a close tag reached only by scanning past
/// whatever attributes an opening tag carries (e.g.
/// `<c:f xml:space="preserve">`, `</c:f >`) rather than by matching a fixed
/// literal string.
///
/// Anything this scanner cannot make sense of — an unterminated comment or
/// CDATA section, a reference element with no matching close tag — is an
/// error, not a best-effort guess: running on past malformed input is how a
/// reference silently attaches to the wrong element.
fn reference_spans(part: &[u8]) -> Result<Vec<Span>, ChartError> {
    let mut spans = Vec::new();
    let mut pos = 0usize;
    while pos < part.len() {
        if part[pos..].starts_with(b"<!--") {
            let rel = find(&part[pos + 4..], b"-->")
                .ok_or(ChartError::UnterminatedComment { at: pos })?;
            pos += 4 + rel + 3;
            continue;
        }
        if part[pos..].starts_with(b"<![CDATA[") {
            let rel =
                find(&part[pos + 9..], b"]]>").ok_or(ChartError::UnterminatedCData { at: pos })?;
            pos += 9 + rel + 3;
            continue;
        }
        if let Some((name_len, close_name)) = reference_open_at(&part[pos..]) {
            let after_name = pos + name_len;
            let Some((tag_end, self_closing)) = scan_tag_end(part, after_name) else {
                return Err(ChartError::UnclosedReference { at: pos });
            };
            if self_closing {
                // Holds no content, so there is nothing to replace and
                // nothing to count.
                pos = tag_end;
                continue;
            }
            let Some((content_end, close_end)) = find_close(part, tag_end, close_name) else {
                return Err(ChartError::UnclosedReference { at: pos });
            };
            spans.push(Span {
                start: tag_end,
                end: content_end,
            });
            pos = close_end;
            continue;
        }
        pos += 1;
    }
    Ok(spans)
}

/// If `bytes` starts with a `<c:f` or `<f` open-tag name at a valid tag
/// boundary (followed by `>`, `/`, or whitespace — never by another letter,
/// which is what tells `<c:f` apart from `<c:formatCode`), returns the
/// matched name's length and the bytes its close tag's name must match.
fn reference_open_at(bytes: &[u8]) -> Option<(usize, &'static [u8])> {
    if bytes.starts_with(b"<c:f") && is_tag_boundary(bytes.get(4)) {
        return Some((4, b"c:f"));
    }
    if bytes.starts_with(b"<f") && is_tag_boundary(bytes.get(2)) {
        return Some((2, b"f"));
    }
    None
}

fn is_tag_boundary(byte: Option<&u8>) -> bool {
    matches!(byte, Some(b'>' | b'/' | b' ' | b'\t' | b'\r' | b'\n'))
}

/// Scans forward from `start` — just after an element's name, e.g. right
/// after `<c:f` — to the `>` that ends its start tag, skipping over quoted
/// attribute values so a `>` inside one (however unlikely in practice) is
/// not mistaken for the tag's end. Returns the index just past that `>`, and
/// whether the tag is self-closing (`/>`).
fn scan_tag_end(bytes: &[u8], start: usize) -> Option<(usize, bool)> {
    let mut i = start;
    let mut in_quote: Option<u8> = None;
    while i < bytes.len() {
        let b = bytes[i];
        match in_quote {
            Some(q) => {
                if b == q {
                    in_quote = None;
                }
            }
            None => match b {
                b'"' | b'\'' => in_quote = Some(b),
                b'>' => {
                    let self_closing = i > start && bytes[i - 1] == b'/';
                    return Some((i + 1, self_closing));
                }
                _ => {}
            },
        }
        i += 1;
    }
    None
}

/// Finds a close tag for `name` (e.g. `b"c:f"`) starting no earlier than
/// `from`, tolerating whitespace between the name and the tag's closing `>`
/// (`</c:f >` is well-formed XML that a literal-needle search would miss).
/// Returns the content's end (the close tag's opening `<`) and the index
/// just past the close tag's `>`.
fn find_close(bytes: &[u8], from: usize, name: &[u8]) -> Option<(usize, usize)> {
    let mut i = from;
    while i < bytes.len() {
        if bytes[i] == b'<' && bytes.get(i + 1) == Some(&b'/') && bytes[i + 2..].starts_with(name) {
            let mut j = i + 2 + name.len();
            while matches!(bytes.get(j), Some(b' ' | b'\t' | b'\r' | b'\n')) {
                j += 1;
            }
            if bytes.get(j) == Some(&b'>') {
                return Some((i, j + 1));
            }
        }
        i += 1;
    }
    None
}

/// The prefix the chart namespace carries in `part`, from its root element:
/// `c:` for an Excel-written part, empty for an excelize-written one that
/// binds the chart namespace as the default. `None` when neither root element
/// is present, which is what makes the bytes not a chart part.
///
/// `<c:chartSpace` does not contain the literal `<chartSpace`, so the two
/// tests cannot both match and the order they are tried in does not matter.
fn chart_prefix(part: &[u8]) -> Option<&'static [u8]> {
    if find(part, b"<c:chartSpace").is_some() {
        Some(b"c:")
    } else if find(part, b"<chartSpace").is_some() {
        Some(b"")
    } else {
        None
    }
}

/// Whether `part` spells its chart-namespace elements two different ways.
///
/// [`chart_prefix`] reads the prefix from the root element and every
/// structural needle is built from it, so an element spelled with the *other*
/// prefix is not merely unrecognised — it is invisible. That is worse than an
/// unknown element: [`title_spans`] bounds its search at the first
/// `<c:plotArea>`/`<plotArea>` precisely so an axis title can never be taken
/// for the chart's, and a bound whose needle matches nothing is skipped, not
/// failed. The first `<title>` in document order is then whatever the document
/// happens to open with, and renaming it returns `Ok` on a chart that is
/// wrong.
///
/// No writer produces such a document, and there is no reading of one that is
/// more right than another, so it is refused rather than guessed at — the same
/// choice this module makes everywhere else it cannot know the answer.
///
/// The two elements checked are the two whose spelling this module depends on:
/// `plotArea` (the bound) and `title` (what the bound protects). References
/// are deliberately not checked, because [`reference_spans`] recognises both
/// spellings wherever they occur.
fn mixes_namespace_prefixes(part: &[u8], prefix: &[u8]) -> bool {
    // `<c:plotArea` does not contain `<plotArea`, and `<c:title>` does not
    // contain `<title>`, so a needle in one spelling cannot match the other.
    let other: &[u8] = if prefix.is_empty() { b"c:" } else { b"" };
    [b"plotArea".as_slice(), b"title>".as_slice()]
        .into_iter()
        .any(|name| find(part, &tag(b"<", other, name)).is_some())
}

/// `<`, `prefix`, `name` — e.g. `<c:title>` or `<title>`.
fn tag(open: &[u8], prefix: &[u8], name: &[u8]) -> Vec<u8> {
    [open, prefix, name].concat()
}

/// Locates the chart's own title, if it has one this crate can replace.
///
/// `prefix` is the chart namespace's prefix in this part, from
/// [`chart_prefix`]; the `a:` runs inside a title are drawingml-main and are
/// prefixed by both writers, so only the chart-namespace needles vary.
///
/// Bounded to the first `<c:title>` that opens before the first
/// `<c:plotArea>` — axis titles always live inside `<c:plotArea>`, so a
/// `<c:title>` found at or after it belongs to an axis, not the chart, and is
/// never returned here. Within that element, every `<a:r>` run is located;
/// if there are none (the title is bound to a cell via `<c:strRef>` rather
/// than holding rich text) this returns `None` rather than searching onward
/// into whatever follows, which used to find an axis title instead.
fn title_spans(part: &[u8], prefix: &[u8]) -> Option<TitleSpans> {
    let title_open = tag(b"<", prefix, b"title>");
    let title_close = tag(b"</", prefix, b"title>");
    let plot_open = tag(b"<", prefix, b"plotArea");

    let title_at = find(part, &title_open)?;
    if let Some(plot_at) = find(part, &plot_open) {
        if title_at >= plot_at {
            return None;
        }
    }
    // If `</c:title>` is missing entirely the template is malformed; treated
    // as "no title to replace" rather than searching past the end of the
    // element, which is the same refuse-rather-than-mangle choice as an
    // empty title.
    let close_rel = find(&part[title_at..], &title_close)?;
    let title_end = title_at + close_rel + title_close.len();
    let region = &part[title_at..title_end];

    let mut runs = Vec::new();
    let mut cursor = 0usize;
    while let Some(open_rel) = find(&region[cursor..], b"<a:r>") {
        let open_at = cursor + open_rel;
        let Some(close_rel) = find(&region[open_at..], b"</a:r>") else {
            break;
        };
        let run_end = open_at + close_rel + b"</a:r>".len();
        runs.push(Span {
            start: title_at + open_at,
            end: title_at + run_end,
        });
        cursor = run_end;
    }

    let first_run = *runs.first()?;
    let first_run_bytes = &part[first_run.start..first_run.end];
    let t_open_rel = find(first_run_bytes, b"<a:t>")?;
    let t_start = first_run.start + t_open_rel + b"<a:t>".len();
    let t_close_rel = find(&part[t_start..], b"</a:t>")?;

    Some(TitleSpans {
        text: Span {
            start: t_start,
            end: t_start + t_close_rel,
        },
        extra_runs: runs[1..].to_vec(),
    })
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChartSpec;

    const PREFIXED: &[u8] = br#"<c:chartSpace><c:ser><c:f>Old!$A$1</c:f></c:ser><c:ser><c:f>Old!$B$1</c:f></c:ser></c:chartSpace>"#;
    const BARE: &[u8] = br#"<c:chartSpace><c:ser><f>Old!$A$1</f></c:ser></c:chartSpace>"#;

    fn rendered(chart: TemplateChart) -> String {
        String::from_utf8(chart.render().expect("a chart").xml).expect("UTF-8")
    }

    #[test]
    fn every_reference_is_replaced_in_document_order() {
        let out = rendered(
            ChartSpec::from_template(PREFIXED)
                .expect("a template")
                .with_references(vec!["New!$A$2".to_string(), "New!$B$2".to_string()]),
        );
        assert_eq!(
            out,
            "<c:chartSpace><c:ser><c:f>New!$A$2</c:f></c:ser><c:ser><c:f>New!$B$2</c:f></c:ser></c:chartSpace>"
        );
    }

    #[test]
    fn an_unprefixed_reference_element_is_handled_too() {
        let out = rendered(
            ChartSpec::from_template(BARE)
                .expect("a template")
                .with_references(vec!["New!$A$2".to_string()]),
        );
        assert_eq!(
            out,
            "<c:chartSpace><c:ser><f>New!$A$2</f></c:ser></c:chartSpace>"
        );
    }

    #[test]
    fn everything_outside_the_reference_elements_is_byte_identical() {
        let template: &[u8] = br#"<c:chartSpace><c:spPr><a:solidFill><a:srgbClr val="4472C4"/></a:solidFill></c:spPr><c:f>Old!$A$1</c:f></c:chartSpace>"#;
        let out = rendered(
            ChartSpec::from_template(template)
                .expect("a template")
                .with_references(vec!["New!$A$1".to_string()]),
        );
        assert!(out.contains(r#"<a:srgbClr val="4472C4"/>"#), "{out}");
    }

    #[test]
    fn restating_the_references_a_template_already_holds_returns_it_unchanged() {
        let out = ChartSpec::from_template(PREFIXED)
            .expect("a template")
            .with_references(vec!["Old!$A$1".to_string(), "Old!$B$1".to_string()])
            .render()
            .expect("a chart");
        assert_eq!(out.xml, PREFIXED);
    }

    #[test]
    fn a_reference_count_mismatch_is_refused_in_both_directions() {
        let too_few = ChartSpec::from_template(PREFIXED)
            .expect("a template")
            .with_references(vec!["New!$A$2".to_string()])
            .render();
        assert!(matches!(
            too_few,
            Err(ChartError::ReferenceCount {
                template: 2,
                supplied: 1
            })
        ));

        let too_many = ChartSpec::from_template(PREFIXED)
            .expect("a template")
            .with_references(vec!["a".to_string(), "b".to_string(), "c".to_string()])
            .render();
        assert!(matches!(
            too_many,
            Err(ChartError::ReferenceCount {
                template: 2,
                supplied: 3
            })
        ));
    }

    #[test]
    fn references_are_escaped_on_the_way_in() {
        let out = rendered(
            ChartSpec::from_template(BARE)
                .expect("a template")
                .with_references(vec!["'A&B'!$A$1".to_string()]),
        );
        assert!(out.contains("<f>&#39;A&amp;B&#39;!$A$1</f>"), "{out}");
    }

    #[test]
    fn the_template_reports_how_many_references_it_wants() {
        let chart = ChartSpec::from_template(PREFIXED).expect("a template");
        assert_eq!(chart.reference_count(), 2);
    }

    #[test]
    fn bytes_that_are_not_a_chart_are_refused() {
        let error = ChartSpec::from_template(b"<worksheet/>");
        assert!(matches!(error, Err(ChartError::NotAChart)));
    }

    #[test]
    fn a_title_can_be_replaced_and_is_escaped() {
        let template: &[u8] = br#"<c:chartSpace><c:title><c:tx><c:rich><a:p><a:r><a:t>Old title</a:t></a:r></a:p></c:rich></c:tx></c:title><c:f>Old!$A$1</c:f></c:chartSpace>"#;
        let out = rendered(
            ChartSpec::from_template(template)
                .expect("a template")
                .with_references(vec!["New!$A$1".to_string()])
                .with_title("R&D by region"),
        );
        assert!(out.contains("<a:t>R&amp;D by region</a:t>"), "{out}");
        assert!(!out.contains("Old title"), "{out}");
    }

    // --- Fix round 1 regressions -------------------------------------------
    //
    // Each test below is a reproducer for one of the six findings from the
    // first review of template mode. Every one of them fails against the
    // logic this crate shipped with at commit 89f9154 — either by panicking,
    // by mis-scanning the template, or by silently mangling output — which is
    // the point: a test that already passed would be guarding nothing.

    #[test]
    fn a_title_span_enclosing_a_reference_span_is_refused_not_a_panic() {
        // Critical 1: `<c:f>` nested inside the title's `<a:t>` makes the
        // reference span and the title span overlap. The old code sorted
        // edits by start and then sliced assuming they were disjoint, which
        // panics (and aborts the process in a release build) on exactly this
        // input.
        let template: &[u8] = br#"<c:chartSpace><c:title><c:tx><c:rich><a:p><a:r><a:t>Old<c:f>X</c:f></a:t></a:r></a:p></c:rich></c:tx></c:title></c:chartSpace>"#;
        let error = ChartSpec::from_template(template)
            .expect("a template")
            .with_references(vec!["New".to_string()])
            .with_title("Hi")
            .render();
        assert!(
            matches!(error, Err(ChartError::OverlappingEdits { .. })),
            "{error:?}"
        );
    }

    const AXIS_TITLE_ONLY: &[u8] = br#"<c:chartSpace><c:chart><c:plotArea><c:valAx><c:title><c:tx><c:rich><a:p><a:r><a:t>Revenue</a:t></a:r></a:p></c:rich></c:tx></c:title></c:valAx></c:plotArea></c:chart><c:f>Old!$A$1</c:f></c:chartSpace>"#;

    #[test]
    fn with_title_never_renames_an_axis_title() {
        // Important 2, case 1: the old `title_span` scanned unbounded to the
        // end of the part and took the first `<a:t>` it found, which is the
        // axis title here (this is exactly what this crate's own render.rs
        // emits for `Axis { title: Some(..) }`). It must be refused, not
        // silently renamed.
        let error = ChartSpec::from_template(AXIS_TITLE_ONLY)
            .expect("a template")
            .with_references(vec!["New!$A$1".to_string()])
            .with_title("Chart title")
            .render();
        assert!(
            matches!(error, Err(ChartError::NoTitleInTemplate)),
            "{error:?}"
        );
        // And leaving the title untouched must not rename the axis either.
        let out = rendered(
            ChartSpec::from_template(AXIS_TITLE_ONLY)
                .expect("a template")
                .with_references(vec!["New!$A$1".to_string()]),
        );
        assert!(out.contains("<a:t>Revenue</a:t>"), "{out}");
    }

    const STRREF_CHART_TITLE_WITH_RICH_AXIS: &[u8] = br#"<c:chartSpace><c:chart><c:title><c:tx><c:strRef><c:f>Sheet1!$B$1</c:f></c:strRef></c:tx></c:title><c:plotArea><c:valAx><c:title><c:tx><c:rich><a:p><a:r><a:t>Revenue</a:t></a:r></a:p></c:rich></c:tx></c:title></c:valAx></c:plotArea></c:chart><c:f>Old!$A$2</c:f></c:chartSpace>"#;

    #[test]
    fn a_strref_chart_title_is_refused_rather_than_renaming_the_axis() {
        // Important 2, case 2: the chart's own title is a cell reference
        // (`<c:strRef>`, no `<a:t>`), so the old unbounded scan fell through
        // to the value axis's rich-text title and renamed that instead.
        let error = ChartSpec::from_template(STRREF_CHART_TITLE_WITH_RICH_AXIS)
            .expect("a template")
            .with_references(vec!["New!$B$1".to_string(), "New!$A$2".to_string()])
            .with_title("Chart title")
            .render();
        assert!(
            matches!(error, Err(ChartError::NoTitleInTemplate)),
            "{error:?}"
        );
    }

    const MULTI_RUN_TITLE: &[u8] = br#"<c:chartSpace><c:title><c:tx><c:rich><a:p><a:r><a:t>Monthly </a:t></a:r><a:r><a:t>totals</a:t></a:r></a:p></c:rich></c:tx></c:title><c:f>Old!$A$1</c:f></c:chartSpace>"#;

    #[test]
    fn a_multi_run_title_is_replaced_without_a_leftover_run() {
        // Important 3: Excel splits a title into several `<a:r>` runs
        // whenever formatting differs partway through it. The old code only
        // replaced the first run's text, leaving the second run's text
        // trailing after it (`"Quarterly totalstotals"`).
        let out = rendered(
            ChartSpec::from_template(MULTI_RUN_TITLE)
                .expect("a template")
                .with_references(vec!["New!$A$1".to_string()])
                .with_title("Quarterly totals"),
        );
        assert!(out.contains("<a:t>Quarterly totals</a:t>"), "{out}");
        assert!(!out.contains("totalstotals"), "{out}");
        assert_eq!(out.matches("<a:r>").count(), 1, "{out}");
    }

    const TRAILING_SPACE_CLOSE: &[u8] =
        br#"<c:chartSpace><c:f>Old!$A$1</c:f ><c:f>Old!$B$1</c:f></c:chartSpace>"#;

    #[test]
    fn a_close_tag_with_trailing_whitespace_does_not_swallow_the_next_element() {
        // Important 4: `</c:f >` is well-formed XML but does not match the
        // literal needle `</c:f>`, so the old scan ran on to the *next*
        // `</c:f>` and swallowed an entire reference element.
        let chart = ChartSpec::from_template(TRAILING_SPACE_CLOSE).expect("a template");
        assert_eq!(chart.reference_count(), 2);
        let out =
            rendered(chart.with_references(vec!["New!$A$1".to_string(), "New!$B$1".to_string()]));
        assert_eq!(
            out,
            "<c:chartSpace><c:f>New!$A$1</c:f ><c:f>New!$B$1</c:f></c:chartSpace>"
        );
    }

    const COMMENTED_OUT_REFERENCE: &[u8] = br#"<c:chartSpace><!-- was <c:f>Dead!$A$1</c:f> --><c:ser><c:f>Old!$A$1</c:f></c:ser></c:chartSpace>"#;

    #[test]
    fn a_reference_inside_a_comment_is_not_counted_or_touched() {
        // Important 5: a `<c:f>` inside a comment is not a live reference.
        // The old scan does not know about comments and counted it anyway,
        // which shifted every later replacement onto the wrong element.
        let chart = ChartSpec::from_template(COMMENTED_OUT_REFERENCE).expect("a template");
        assert_eq!(chart.reference_count(), 1);
        let out = rendered(chart.with_references(vec!["New!$A$1".to_string()]));
        assert!(out.contains("<!-- was <c:f>Dead!$A$1</c:f> -->"), "{out}");
        assert!(out.contains("<c:ser><c:f>New!$A$1</c:f></c:ser>"), "{out}");
    }

    const ATTRIBUTE_REFERENCE: &[u8] =
        br#"<c:chartSpace><c:f xml:space="preserve">Old!$A$1</c:f></c:chartSpace>"#;

    #[test]
    fn a_reference_element_with_an_attribute_is_still_recognised() {
        // Important 6, case 1: the old needle was the literal `<c:f>`, so an
        // attribute on the element (`xml:space="preserve"`) made it invisible
        // and its stale reference survived into the output.
        let out = rendered(
            ChartSpec::from_template(ATTRIBUTE_REFERENCE)
                .expect("a template")
                .with_references(vec!["New!$A$1".to_string()]),
        );
        assert_eq!(
            out,
            r#"<c:chartSpace><c:f xml:space="preserve">New!$A$1</c:f></c:chartSpace>"#
        );
    }

    const SELF_CLOSING_REFERENCE: &[u8] =
        br#"<c:chartSpace><c:f/><c:f>Old!$A$1</c:f></c:chartSpace>"#;

    #[test]
    fn a_self_closing_reference_element_is_skipped_not_miscounted() {
        // Important 6, case 2: a self-closing `<c:f/>` holds no reference to
        // replace. It must be skipped deliberately (and not counted), rather
        // than left for the old literal-needle scan to mishandle.
        let chart = ChartSpec::from_template(SELF_CLOSING_REFERENCE).expect("a template");
        assert_eq!(chart.reference_count(), 1);
        let out = rendered(chart.with_references(vec!["New!$A$1".to_string()]));
        assert_eq!(
            out,
            "<c:chartSpace><c:f/><c:f>New!$A$1</c:f></c:chartSpace>"
        );
    }

    #[test]
    fn an_element_whose_name_merely_starts_with_f_is_not_a_reference() {
        // The `>`-terminated needles the first implementation used excluded
        // `<c:formatCode>` by accident of spelling. Matching a tag boundary
        // instead of a literal `>` is what keeps that exclusion deliberate,
        // and real chart parts carry `<c:formatCode>` inside every
        // `<c:numCache>`. Substituting into one would corrupt the number
        // format silently, so this stays pinned.
        let template: &[u8] = br#"<c:chartSpace><c:numCache><c:formatCode>General</c:formatCode></c:numCache><c:f>Old!$A$1</c:f></c:chartSpace>"#;
        let chart = ChartSpec::from_template(template).expect("a template");
        assert_eq!(chart.reference_count(), 1);
        let out = rendered(chart.with_references(vec!["New!$A$1".to_string()]));
        assert!(
            out.contains("<c:formatCode>General</c:formatCode>"),
            "{out}"
        );
        assert!(out.contains("<c:f>New!$A$1</c:f>"), "{out}");
    }

    #[test]
    fn replacing_a_title_a_template_does_not_have_is_refused() {
        let error = ChartSpec::from_template(BARE)
            .expect("a template")
            .with_references(vec!["New!$A$1".to_string()])
            .with_title("anything")
            .render();
        assert!(matches!(error, Err(ChartError::NoTitleInTemplate)));
    }

    #[test]
    fn the_rendered_part_declares_the_chart_content_type() {
        let part = ChartSpec::from_template(BARE)
            .expect("a template")
            .with_references(vec!["New!$A$1".to_string()])
            .render()
            .expect("a chart");
        assert_eq!(part.content_type, crate::ChartPart::CONTENT_TYPE);
    }

    // --- The excelize spelling ---------------------------------------------
    //
    // excelize binds the chart namespace as the document default and so
    // prefixes nothing: `<chartSpace>`, `<title>`, `<plotArea>`, `<f>`. A
    // corpus audit of 366 chart parts across 54 excelize-written workbooks
    // found 366 of 366 spelled that way, so this is the shape a chart part
    // arrives in far more often than Excel's own. Before these tests,
    // template mode recognised only the `c:` spelling and refused every such
    // part outright with `NotAChart`. The fixture below is shaped like one of
    // them: default-namespaced chart elements, `a:`-prefixed drawingml runs,
    // `<v>`-free.

    /// An excelize-shaped chart part: chart namespace as the default, a rich
    /// chart title, an axis title inside `<plotArea>`, and two references.
    const DEFAULT_NAMESPACED: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<chartSpace xmlns="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><chart><title><tx><rich><a:p><a:r><a:t>Old chart title</a:t></a:r></a:p></rich></tx></title><plotArea><valAx><title><tx><rich><a:p><a:r><a:t>Axis</a:t></a:r></a:p></rich></tx></title></valAx><ser><f>Old!$A$1</f></ser><ser><f>Old!$B$1</f></ser></plotArea></chart></chartSpace>"#;

    #[test]
    fn an_excelize_written_part_is_recognised_as_a_chart() {
        let chart = ChartSpec::from_template(DEFAULT_NAMESPACED).expect("a template");
        assert_eq!(chart.reference_count(), 2);
    }

    #[test]
    fn an_excelize_written_parts_own_title_is_the_one_replaced() {
        let out = rendered(
            ChartSpec::from_template(DEFAULT_NAMESPACED)
                .expect("a template")
                .with_references(vec!["New!$A$2".to_string(), "New!$B$2".to_string()])
                .with_title("Example OPA Findings by week"),
        );
        assert!(
            out.contains("<a:t>Example OPA Findings by week</a:t>"),
            "{out}"
        );
        assert!(!out.contains("Old chart title"), "{out}");
        // The axis title lives inside `<plotArea>` and is not the chart's.
        // Without a prefix there is nothing but that ordering to tell them
        // apart, which is exactly what makes this worth pinning.
        assert!(out.contains("<a:t>Axis</a:t>"), "{out}");
        assert!(out.contains("<f>New!$A$2</f>"), "{out}");
        assert!(out.contains("<f>New!$B$2</f>"), "{out}");
    }

    #[test]
    fn an_excelize_written_part_with_only_an_axis_title_is_refused() {
        // The `<plotArea>` bound has to hold for the unprefixed spelling too:
        // renaming an axis to a chart's title is the silently-wrong outcome
        // this whole bound exists to prevent.
        let axis_only: &[u8] = br#"<chartSpace xmlns="http://schemas.openxmlformats.org/drawingml/2006/chart"><chart><plotArea><valAx><title><tx><rich><a:p><a:r><a:t>Revenue</a:t></a:r></a:p></rich></tx></title></valAx><ser><f>Old!$A$1</f></ser></plotArea></chart></chartSpace>"#;
        let error = ChartSpec::from_template(axis_only)
            .expect("a template")
            .with_references(vec!["New!$A$1".to_string()])
            .with_title("anything")
            .render();
        assert!(
            matches!(error, Err(ChartError::NoTitleInTemplate)),
            "{error:?}"
        );
    }

    // --- Mixed spellings, in both directions -------------------------------
    //
    // The prefix comes from the root element and is not re-guessed per
    // element, so a document that spells its chart elements both ways is not
    // one this module can read: the needle built from the root matches
    // nothing in the other spelling, and a needle that matches nothing is
    // skipped rather than failed. Both directions are refused at
    // `from_template`, before a caller can ask for a title at all.

    #[test]
    fn a_prefixed_root_over_an_unprefixed_title_is_refused() {
        // The benign direction: the bound (`<c:plotArea>`) is absent
        // altogether, so nothing is mis-bounded and the worst outcome without
        // the check is a title reported as missing. Refused anyway, because
        // "prefixed root, unprefixed title" is one reading among several and
        // this module does not pick between readings.
        let mixed: &[u8] = br#"<c:chartSpace><title><tx><rich><a:p><a:r><a:t>Old</a:t></a:r></a:p></rich></tx></title><c:f>Old!$A$1</c:f></c:chartSpace>"#;
        let error = ChartSpec::from_template(mixed).err();
        assert!(
            matches!(error, Some(ChartError::MixedNamespacePrefixes)),
            "{error:?}"
        );
    }

    #[test]
    fn an_unprefixed_root_over_a_prefixed_plot_area_is_refused_not_silently_mis_titled() {
        // The hostile direction, and the reason this is an error rather than
        // a doc caveat. The root is bare, so `title_spans` looks for
        // `<plotArea` — which a `c:`-prefixed plot area does not contain. The
        // bound is skipped, the first `<title>` in document order is taken,
        // and here that is the *axis* title: `with_title` would rename the
        // axis to the chart's intended title, leave the chart's own title
        // untouched, and return `Ok`. A refusal is the only answer that is
        // not a plausible lie.
        let mixed: &[u8] = br#"<chartSpace xmlns="http://schemas.openxmlformats.org/drawingml/2006/chart"><chart><c:plotArea><valAx><title><tx><rich><a:p><a:r><a:t>Revenue</a:t></a:r></a:p></rich></tx></title></valAx><ser><f>Old!$A$1</f></ser></c:plotArea><title><tx><rich><a:p><a:r><a:t>Old chart title</a:t></a:r></a:p></rich></tx></title></chart></chartSpace>"#;
        let error = ChartSpec::from_template(mixed).err();
        assert!(
            matches!(error, Some(ChartError::MixedNamespacePrefixes)),
            "{error:?}"
        );
    }

    #[test]
    fn a_consistently_spelled_part_is_not_mistaken_for_a_mixed_one() {
        // The check must not fire on either spelling used on its own — a
        // false positive here would refuse every real chart part. References
        // are deliberately outside its scope: `BARE` is a `c:`-rooted part
        // holding an unprefixed `<f>`, which `reference_spans` recognises on
        // purpose.
        for part in [PREFIXED, BARE, DEFAULT_NAMESPACED, AXIS_TITLE_ONLY] {
            assert!(
                ChartSpec::from_template(part).is_ok(),
                "refused a consistently spelled part"
            );
        }
    }
}
