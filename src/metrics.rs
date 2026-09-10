//! Converting a chart's pixel size into the cell anchor OOXML stores.
//!
//! # Why a chart's size is not stored
//!
//! A `twoCellAnchor` chart has no width and height. It has two cell positions,
//! and its size is whatever the distance between them happens to be. So a sheet
//! that gains a row inside a chart's span makes that chart taller, and keeping
//! a chart the same size across an edit means recomputing where it ends.
//!
//! # Why the row table is yours to supply
//!
//! How many pixels a row occupies is a property of the workbook, not of the
//! format. One project measured its own table across 488 real anchors and found
//! that the obvious rule — `floor(points * 1.2)` — fits four of its five heights
//! and is wrong on the fifth by three pixels, which is enough, compounded over a
//! chart's span, to walk it into the band below. So the table is configuration,
//! and [`UnknownHeight::Refuse`] exists for callers who would rather fail than
//! guess.
//!
//! # Why every step below is checked arithmetic
//!
//! Every number the walk touches — the chart's size, the starting offset, the
//! EMU-per-pixel conversion, a row or column's pixel width — comes from a
//! caller. A zero `emu_per_pixel` would make the walk divide by zero; a row
//! or column that never reports a nonzero width would make it loop, in the
//! worst case, until a `u32` row or column index overflows; an EMU-per-pixel
//! far from Excel's own would make the final multiplication overflow. None
//! of those are reachable with [`RowMetrics::excel_default`] and a real
//! worksheet, but this crate has no way to know the caller stayed inside
//! that range, so every arithmetic step is checked, the walk gives up once
//! it has gone further than any real worksheet extends, and every failure
//! becomes a [`ChartError`] rather than a panic.

use crate::error::ChartError;

/// One past the highest column index a worksheet can have (`XFD`, the
/// 16,384th column). A walk that has not found its fit by here is not
/// modelling a real worksheet.
const COLUMN_INDEX_LIMIT: u32 = 16_384;

/// One past the highest row index a worksheet can have (row 1,048,576,
/// zero-based). A walk that has not found its fit by here is not modelling a
/// real worksheet.
const ROW_INDEX_LIMIT: u32 = 1_048_576;

/// What to do with a row height the table does not cover.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum UnknownHeight {
    /// Return [`ChartError::UnmeasuredRowHeight`].
    ///
    /// Choose this when a wrong answer is worse than no answer. Guessing moves
    /// a chart's bottom edge by a few pixels per row and nothing looks wrong
    /// until it overlaps something.
    Refuse,
    /// Use `floor(points * 1.2)`, Excel's nominal points-to-pixels ratio.
    Interpolate,
}

/// How rows and pixels relate in the sheet a chart is anchored to.
#[derive(Debug, Clone)]
pub struct RowMetrics {
    emu_per_pixel: i64,
    default_row_pixels: u32,
    measured: Vec<(u32, u32)>,
    unknown: UnknownHeight,
}

impl RowMetrics {
    /// 20 px default rows, 9525 EMU per pixel, unknown heights interpolated.
    ///
    /// 9525 is not a convention: it is 914400 EMU per inch over 96 pixels per
    /// inch, and every offset in every anchor measured against this crate is a
    /// whole multiple of it.
    #[must_use]
    pub fn excel_default() -> Self {
        Self {
            emu_per_pixel: 9525,
            default_row_pixels: 20,
            measured: Vec::new(),
            unknown: UnknownHeight::Interpolate,
        }
    }

    /// Overrides the EMU-per-pixel conversion.
    ///
    /// Zero is refused by [`two_cell_anchor`] with
    /// [`ChartError::ZeroEmuPerPixel`] rather than dividing by it; it is
    /// accepted here because the builder itself cannot fail.
    #[must_use]
    pub fn emu_per_pixel(mut self, emu: i64) -> Self {
        self.emu_per_pixel = emu;
        self
    }

    /// Sets the height of a row carrying no explicit height.
    #[must_use]
    pub fn default_row_pixels(mut self, pixels: u32) -> Self {
        self.default_row_pixels = pixels;
        self
    }

    /// Records a measured `(points, pixels)` pair. A later call for the same
    /// point height replaces the earlier one.
    #[must_use]
    pub fn measured(mut self, points: u32, pixels: u32) -> Self {
        self.measured.retain(|(existing, _)| *existing != points);
        self.measured.push((points, pixels));
        self
    }

    /// Sets the policy for heights the table does not cover.
    #[must_use]
    pub fn unknown(mut self, policy: UnknownHeight) -> Self {
        self.unknown = policy;
        self
    }

    /// How many pixels a row of this height occupies. `None` means the row
    /// carries no explicit height.
    ///
    /// # Errors
    ///
    /// [`ChartError::UnmeasuredRowHeight`] if the height is not in the table
    /// and the policy is [`UnknownHeight::Refuse`].
    ///
    /// [`ChartError::AnchorArithmeticOverflow`] if the policy is
    /// [`UnknownHeight::Interpolate`] and `points * 12` overflows `u32` —
    /// unreachable for any point size a worksheet can display, but a caller
    /// could construct one.
    pub fn pixels_for(&self, points: Option<u32>) -> Result<u32, ChartError> {
        let Some(points) = points else {
            return Ok(self.default_row_pixels);
        };
        if let Some((_, pixels)) = self.measured.iter().find(|(pt, _)| *pt == points) {
            return Ok(*pixels);
        }
        match self.unknown {
            UnknownHeight::Refuse => Err(ChartError::UnmeasuredRowHeight { points }),
            UnknownHeight::Interpolate => points.checked_mul(12).map(|scaled| scaled / 10).ok_or(
                ChartError::AnchorArithmeticOverflow {
                    context: "row height interpolation",
                },
            ),
        }
    }
}

/// A position in the sheet: a cell, and an offset into it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellAnchor {
    /// Zero-based column, as the drawing part writes it.
    pub col: u32,
    /// Offset into that column, in EMU.
    pub col_offset_emu: i64,
    /// Zero-based row, as the drawing part writes it.
    pub row: u32,
    /// Offset into that row, in EMU.
    pub row_offset_emu: i64,
}

/// Where a chart starts and where it ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TwoCellAnchor {
    /// The top-left corner. Supplied by the caller and never moved.
    pub from: CellAnchor,
    /// The bottom-right corner, computed from the chart's pixel size.
    pub to: CellAnchor,
}

/// Walks one axis (columns or rows) from `start_index`/`start_offset_emu`
/// until `size_px` pixels are consumed, returning the index and remaining
/// offset the walk lands on.
///
/// `pixels_at(i)` supplies the pixel width or height of position `i`; for
/// columns that is `i` itself, for rows it is `i + 1` (see the note on
/// [`two_cell_anchor`]'s two closures).
///
/// This is the same walk described inline for [`two_cell_anchor`] — starting
/// `remaining` at the size plus the starting offset in pixels, consuming one
/// position's worth per iteration, and stopping the first time `remaining` is
/// strictly less than what the current position holds — except every step is
/// a checked operation, and `index_limit` bounds how far the walk goes, so a
/// caller-supplied zero, overflow, or position that never reports a nonzero
/// size turns into a [`ChartError`] instead of a panic or an unbounded loop.
fn walk(
    start_index: u32,
    start_offset_emu: i64,
    size_px: u32,
    emu_per_pixel: i64,
    index_limit: u32,
    context: &'static str,
    mut pixels_at: impl FnMut(u32) -> Result<u32, ChartError>,
) -> Result<(u32, i64), ChartError> {
    if emu_per_pixel == 0 {
        return Err(ChartError::ZeroEmuPerPixel);
    }
    let overflow = || ChartError::AnchorArithmeticOverflow { context };
    let span_exceeded = || ChartError::AnchorSpanExceedsWorksheet { context };

    let start_offset_px = start_offset_emu
        .checked_div(emu_per_pixel)
        .ok_or_else(overflow)?;
    let mut remaining = i64::from(size_px)
        .checked_add(start_offset_px)
        .ok_or_else(overflow)?;
    let mut index = start_index;
    loop {
        if index >= index_limit {
            return Err(span_exceeded());
        }
        let pixels = i64::from(pixels_at(index)?);
        if remaining < pixels {
            let offset_emu = remaining.checked_mul(emu_per_pixel).ok_or_else(overflow)?;
            return Ok((index, offset_emu));
        }
        remaining = remaining.checked_sub(pixels).ok_or_else(overflow)?;
        index = index.checked_add(1).ok_or_else(overflow)?;
    }
}

/// Computes the bottom-right corner of a chart `size_px` wide and tall.
///
/// `row_height_points` is asked for **one-based** rows, the way a worksheet
/// numbers them, and returns `None` for a row with no explicit height.
/// `col_width_px` is asked for **zero-based** columns, the way the drawing part
/// numbers them, and returns `None` for a column whose width is unknown.
///
/// The asymmetry is deliberate and matches how the two are addressed in the
/// parts themselves; getting it wrong shifts a chart by exactly one row.
///
/// # Errors
///
/// [`ChartError::UnmeasuredRowHeight`] or [`ChartError::UnmeasuredColumnWidth`]
/// if any row or column in the span cannot be sized.
///
/// [`ChartError::ZeroEmuPerPixel`] if `metrics`'s EMU-per-pixel conversion is
/// zero; [`ChartError::AnchorArithmeticOverflow`] if the walk's arithmetic
/// overflows; or [`ChartError::AnchorSpanExceedsWorksheet`] if it walks past
/// the largest row or column a worksheet can have without finding a fit. All
/// three are refused rather than panicking, since every number here can come
/// from a caller.
pub fn two_cell_anchor(
    from: CellAnchor,
    size_px: (u32, u32),
    metrics: &RowMetrics,
    row_height_points: impl Fn(u32) -> Option<u32>,
    col_width_px: impl Fn(u32) -> Option<u32>,
) -> Result<TwoCellAnchor, ChartError> {
    let (width_px, height_px) = size_px;

    let (to_col, to_col_offset) = walk(
        from.col,
        from.col_offset_emu,
        width_px,
        metrics.emu_per_pixel,
        COLUMN_INDEX_LIMIT,
        "column",
        |col| col_width_px(col).ok_or(ChartError::UnmeasuredColumnWidth { column: col }),
    )?;

    let (to_row, to_row_offset) = walk(
        from.row,
        from.row_offset_emu,
        height_px,
        metrics.emu_per_pixel,
        ROW_INDEX_LIMIT,
        "row",
        |row| metrics.pixels_for(row_height_points(row + 1)),
    )?;

    Ok(TwoCellAnchor {
        from,
        to: CellAnchor {
            col: to_col,
            col_offset_emu: to_col_offset,
            row: to_row,
            row_offset_emu: to_row_offset,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A table built from measured anchors rather than the 1.2 formula, the
    /// way a caller with a real workbook in hand would configure it.
    fn measured_metrics() -> RowMetrics {
        RowMetrics::excel_default()
            .default_row_pixels(20)
            .measured(26, 31)
            .measured(30, 36)
            .measured(32, 38)
            .measured(42, 50)
            .measured(168, 198)
            .unknown(UnknownHeight::Refuse)
    }

    fn top() -> CellAnchor {
        CellAnchor {
            col: 0,
            col_offset_emu: 0,
            row: 0,
            row_offset_emu: 0,
        }
    }

    #[test]
    fn a_chart_ending_exactly_on_a_boundary_takes_the_next_row_at_offset_zero() {
        let anchor = two_cell_anchor(
            top(),
            (100, 80),
            &measured_metrics(),
            |_| None,
            |_| Some(64),
        )
        .expect("an anchor");
        assert_eq!(anchor.to.row, 4);
        assert_eq!(anchor.to.row_offset_emu, 0);
    }

    #[test]
    fn a_chart_ending_part_way_down_a_row_keeps_the_remainder_as_an_offset() {
        let anchor = two_cell_anchor(
            top(),
            (100, 85),
            &measured_metrics(),
            |_| None,
            |_| Some(64),
        )
        .expect("an anchor");
        assert_eq!(anchor.to.row, 4);
        assert_eq!(anchor.to.row_offset_emu, 5 * 9525);
    }

    #[test]
    fn the_starting_offset_counts_towards_the_height() {
        let from = CellAnchor {
            row_offset_emu: 10 * 9525,
            ..top()
        };
        // 80 px of chart starting 10 px down a 20 px row ends 90 px from the
        // top of row 0, i.e. half way down row 4.
        let anchor = two_cell_anchor(from, (100, 80), &measured_metrics(), |_| None, |_| Some(64))
            .expect("an anchor");
        assert_eq!(anchor.to.row, 4);
        assert_eq!(anchor.to.row_offset_emu, 10 * 9525);
    }

    #[test]
    fn a_measured_row_height_uses_its_measurement_not_a_formula() {
        // 168 points measures 198 px, where floor(168 * 1.2) would give 201.
        let anchor = two_cell_anchor(
            top(),
            (100, 198),
            &measured_metrics(),
            |row| if row == 1 { Some(168) } else { None },
            |_| Some(64),
        )
        .expect("an anchor");
        assert_eq!(anchor.to.row, 1);
        assert_eq!(anchor.to.row_offset_emu, 0);
    }

    #[test]
    fn an_unmeasured_height_is_refused_when_the_policy_says_refuse() {
        let error = two_cell_anchor(
            top(),
            (100, 500),
            &measured_metrics(),
            |_| Some(99),
            |_| Some(64),
        );
        assert!(matches!(
            error,
            Err(ChartError::UnmeasuredRowHeight { points: 99 })
        ));
    }

    #[test]
    fn an_unmeasured_height_is_interpolated_when_the_policy_allows_it() {
        let metrics = RowMetrics::excel_default().unknown(UnknownHeight::Interpolate);
        // floor(50 * 1.2) == 60, so two such rows are 120 px.
        let anchor = two_cell_anchor(top(), (100, 120), &metrics, |_| Some(50), |_| Some(64))
            .expect("an anchor");
        assert_eq!(anchor.to.row, 2);
        assert_eq!(anchor.to.row_offset_emu, 0);
    }

    #[test]
    fn a_row_with_no_height_of_its_own_takes_the_default() {
        let metrics = RowMetrics::excel_default().default_row_pixels(20);
        let anchor =
            two_cell_anchor(top(), (100, 60), &metrics, |_| None, |_| Some(64)).expect("an anchor");
        assert_eq!(anchor.to.row, 3);
    }

    #[test]
    fn columns_are_walked_the_same_way_as_rows() {
        // Four 64 px columns is exactly 256.
        let anchor = two_cell_anchor(
            top(),
            (256, 20),
            &measured_metrics(),
            |_| None,
            |_| Some(64),
        )
        .expect("an anchor");
        assert_eq!(anchor.to.col, 4);
        assert_eq!(anchor.to.col_offset_emu, 0);
    }

    #[test]
    fn a_column_with_no_width_is_refused() {
        let error = two_cell_anchor(top(), (100, 20), &measured_metrics(), |_| None, |_| None);
        assert!(matches!(
            error,
            Err(ChartError::UnmeasuredColumnWidth { column: 0 })
        ));
    }

    #[test]
    fn the_from_corner_is_carried_through_untouched() {
        let from = CellAnchor {
            col: 12,
            col_offset_emu: 3 * 9525,
            row: 7,
            row_offset_emu: 0,
        };
        let anchor = two_cell_anchor(from, (100, 40), &measured_metrics(), |_| None, |_| Some(64))
            .expect("an anchor");
        assert_eq!(anchor.from, from);
    }

    #[test]
    fn a_non_default_emu_per_pixel_scales_the_offsets() {
        let metrics = RowMetrics::excel_default().emu_per_pixel(12700);
        let anchor =
            two_cell_anchor(top(), (100, 85), &metrics, |_| None, |_| Some(64)).expect("an anchor");
        assert_eq!(anchor.to.row_offset_emu, 5 * 12700);
    }

    #[test]
    fn a_zero_emu_per_pixel_is_refused_instead_of_dividing_by_it() {
        let metrics = RowMetrics::excel_default().emu_per_pixel(0);
        let error = two_cell_anchor(top(), (100, 80), &metrics, |_| None, |_| Some(64));
        assert!(matches!(error, Err(ChartError::ZeroEmuPerPixel)));
    }

    #[test]
    fn a_zero_default_row_height_does_not_loop_forever_and_is_refused() {
        // With every row reporting the default height and the default set to
        // zero, no row ever consumes any of `remaining`, so the walk would
        // otherwise run until a `u32` row index overflowed. It must instead
        // stop at `ROW_INDEX_LIMIT` and report the span as too large, quickly
        // rather than after billions of iterations.
        let metrics = RowMetrics::excel_default().default_row_pixels(0);
        let error = two_cell_anchor(top(), (100, 80), &metrics, |_| None, |_| Some(64));
        assert!(matches!(
            error,
            Err(ChartError::AnchorSpanExceedsWorksheet { context: "row" })
        ));
    }

    #[test]
    fn a_column_reporting_zero_width_forever_does_not_loop_forever_and_is_refused() {
        // A column width of zero is realistic on its own — a hidden column —
        // but if every column from here on reports zero, the walk can never
        // consume any of `remaining`. It must stop at `COLUMN_INDEX_LIMIT`
        // rather than walking until a `u32` column index overflowed.
        let error = two_cell_anchor(top(), (100, 20), &measured_metrics(), |_| None, |_| Some(0));
        assert!(matches!(
            error,
            Err(ChartError::AnchorSpanExceedsWorksheet { context: "column" })
        ));
    }

    #[test]
    fn row_height_interpolation_overflow_is_refused_not_panicked() {
        let metrics = RowMetrics::excel_default().unknown(UnknownHeight::Interpolate);
        assert!(matches!(
            metrics.pixels_for(Some(u32::MAX)),
            Err(ChartError::AnchorArithmeticOverflow {
                context: "row height interpolation"
            })
        ));
    }
}
