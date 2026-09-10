//! The one error type this crate returns.

/// Everything that can go wrong authoring a chart.
///
/// `#[non_exhaustive]` so adding a variant later is not a breaking change for
/// downstream crates that match on it.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ChartError {
    /// A chart was rendered with no series.
    #[error("a chart needs at least one series")]
    NoSeries,

    /// Template mode was given a different number of references than the
    /// template holds `<c:f>` elements.
    ///
    /// This is refused rather than best-effort matched: a chart left plotting
    /// the template's ranges looks entirely normal and is wrong.
    #[error(
        "the template holds {template} <c:f> elements and this chart has {supplied} references"
    )]
    ReferenceCount {
        /// `<c:f>` elements found in the template.
        template: usize,
        /// References the caller supplied.
        supplied: usize,
    },

    /// The bytes handed to template mode are not a chart part.
    ///
    /// Both spellings of the root element are accepted — `<c:chartSpace>` from
    /// Excel and `<chartSpace>` from excelize, which binds the chart namespace
    /// as the default — so this really does mean neither is present.
    #[error("the template is not a chart part: no <c:chartSpace> or <chartSpace> element")]
    NotAChart,

    /// The template spells its chart-namespace elements two different ways —
    /// a `c:`-prefixed root over a bare `<plotArea>`, or a bare root over a
    /// `<c:plotArea>`.
    ///
    /// Template mode reads the prefix from the root element once and builds
    /// every structural needle from it, so an element in the other spelling is
    /// invisible rather than merely unrecognised: the bound that keeps an axis
    /// title from being mistaken for the chart's would match nothing, be
    /// skipped, and let [`crate::TemplateChart::with_title`] rename the wrong
    /// element and return success. No writer produces a document like this and
    /// no reading of one is more right than another, so it is refused.
    #[error(
        "the template mixes prefixed and unprefixed chart elements, so which one is the chart's \
         own title cannot be decided"
    )]
    MixedNamespacePrefixes,

    /// Template mode was asked to replace a title the template does not have.
    ///
    /// This covers three cases, all refused the same way because none of
    /// them give template mode a chart title it can safely edit: the
    /// template has no `<c:title>` at all; its only `<c:title>` belongs to
    /// an axis rather than the chart (axis titles live inside
    /// `<c:plotArea>`, which always follows the chart title); or the chart's
    /// `<c:title>` holds no plain-text run to replace, because it is bound
    /// to a cell via `<c:strRef>` instead of holding rich text.
    ///
    /// The element is named here with its `c:` prefix; a template that spells
    /// it `<title>` is read the same way, from the prefix its root element
    /// carries.
    #[error("the template has no <c:title> to replace")]
    NoTitleInTemplate,

    /// Two of template mode's edits to the template overlap, so applying
    /// both would corrupt one of them — most often because a `<c:f>`
    /// reference element is nested inside the chart title's text, which
    /// [`crate::TemplateChart::render`] cannot apply as two independent
    /// substitutions without corrupting one of them.
    #[error(
        "a template edit at byte {at} overlaps the previous one, which ends at byte {previous_end}"
    )]
    OverlappingEdits {
        /// Byte offset where the overlapping edit begins.
        at: usize,
        /// Byte offset where the preceding edit ends.
        previous_end: usize,
    },

    /// A `<!--` comment in the template was never closed with `-->`.
    #[error("an XML comment starting at byte {at} is never closed")]
    UnterminatedComment {
        /// Byte offset where the unclosed comment opens.
        at: usize,
    },

    /// A `<![CDATA[` section in the template was never closed with `]]>`.
    #[error("a CDATA section starting at byte {at} is never closed")]
    UnterminatedCData {
        /// Byte offset where the unclosed CDATA section opens.
        at: usize,
    },

    /// A `<c:f>` or `<f>` element in the template was opened but has no
    /// matching close tag. Refused rather than left to a best-effort scan
    /// that would otherwise run on into whatever follows and either lose an
    /// element or attribute a stray reference to the wrong series.
    #[error("a reference element starting at byte {at} has no matching close tag")]
    UnclosedReference {
        /// Byte offset where the unclosed element opens.
        at: usize,
    },

    /// A row in a chart's span has a height the metrics table does not cover,
    /// and the policy is [`crate::UnknownHeight::Refuse`].
    #[error(
        "no measurement for a {points} point row, so a chart anchored across one cannot be placed"
    )]
    UnmeasuredRowHeight {
        /// The row height, in points.
        points: u32,
    },

    /// A column in a chart's span has no width available.
    #[error("no width for column {column}, so a chart spanning it cannot be placed")]
    UnmeasuredColumnWidth {
        /// Zero-based column index.
        column: u32,
    },

    /// [`crate::RowMetrics::emu_per_pixel`] was set to zero.
    ///
    /// [`crate::two_cell_anchor`] divides an EMU offset by this value to
    /// recover a pixel count, so a zero conversion factor can never be
    /// honoured — there is no pixel count that, times zero, gives back a
    /// nonzero offset.
    #[error("the EMU-per-pixel conversion is zero, so an offset cannot be converted to pixels")]
    ZeroEmuPerPixel,

    /// Placing a chart overflowed 64-bit or 32-bit arithmetic.
    ///
    /// Unreachable with [`crate::RowMetrics::excel_default`]'s conversion
    /// factor and a chart size a worksheet could actually display. It exists
    /// so a caller-supplied `emu_per_pixel`, row height, column width, or
    /// chart size far outside that range is refused instead of silently
    /// wrapping or panicking.
    #[error("arithmetic placing a chart's {context} overflowed")]
    AnchorArithmeticOverflow {
        /// What the overflowing arithmetic was computing: `"column"`,
        /// `"row"`, or `"row height interpolation"`.
        context: &'static str,
    },

    /// The walk computing a chart's anchor passed the largest row or column a
    /// worksheet can actually have without ever finding one that consumed any
    /// of the chart's remaining size.
    ///
    /// This is the guard against a `row_height_points`, `col_width_px`, or
    /// [`crate::RowMetrics::default_row_pixels`] that reports zero forever:
    /// rather than loop for up to four billion iterations before the row or
    /// column index overflows `u32`, the walk gives up once it has gone
    /// further than any real worksheet extends.
    #[error(
        "the anchor's {context} walk passed the largest a worksheet can have without finding a fit"
    )]
    AnchorSpanExceedsWorksheet {
        /// `"column"` or `"row"`.
        context: &'static str,
    },
}
