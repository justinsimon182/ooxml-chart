//! A chart on a sheet of its own.
//!
//! A chartsheet is a workbook sheet that holds one chart and no cells. Its
//! part is a thin wrapper around a drawing, so the chart part and the drawing
//! are the ones made elsewhere in this crate; this module writes the wrapper
//! and the lines that register it in the workbook.

use crate::drawing::{drawing_part_with, Anchor, GraphicFrame};
use crate::error::ChartError;
use crate::xml::escape;

/// The content type a chartsheet part declares in `[Content_Types].xml`.
pub const CHARTSHEET_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.chartsheet+xml";

/// The relationship type a workbook uses to reach a chartsheet.
pub const CHARTSHEET_RELATIONSHIP_TYPE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/chartsheet";

/// The size Excel gives a chartsheet's chart, in EMU.
const CHARTSHEET_WIDTH_EMU: u64 = 9_293_679;
const CHARTSHEET_HEIGHT_EMU: u64 = 6_068_786;

/// A complete chartsheet part whose drawing is reached by the relationship
/// `drawing_relationship_id` (on the chartsheet's own `_rels`).
#[must_use]
pub fn chartsheet_part(drawing_relationship_id: &str) -> Vec<u8> {
    format!(
        concat!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
            r#"<chartsheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main""#,
            r#" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#,
            r#"<sheetViews><sheetView workbookViewId="0" zoomToFit="1"/></sheetViews>"#,
            r#"<pageMargins left="0.7" right="0.7" top="0.75" bottom="0.75" header="0.3" footer="0.3"/>"#,
            r#"<drawing r:id="{}"/></chartsheet>"#,
        ),
        escape(drawing_relationship_id)
    )
    .into_bytes()
}

/// The drawing a chartsheet carries: one chart filling the sheet, anchored the
/// way Excel anchors it (absolutely, at the origin).
#[must_use]
pub fn chartsheet_drawing_part(frame: &GraphicFrame) -> Vec<u8> {
    drawing_part_with(&[(
        Anchor::Absolute {
            x_emu: 0,
            y_emu: 0,
            width_emu: CHARTSHEET_WIDTH_EMU,
            height_emu: CHARTSHEET_HEIGHT_EMU,
        },
        frame.clone(),
    )])
}

/// The `[Content_Types].xml` entry for a chartsheet part, e.g. `part_name`
/// `"/xl/chartsheets/sheet1.xml"`.
#[must_use]
pub fn chartsheet_content_types_override(part_name: &str) -> String {
    format!(
        r#"<Override PartName="{}" ContentType="{CHARTSHEET_CONTENT_TYPE}"/>"#,
        escape(part_name),
    )
}

/// The workbook's `_rels` entry that reaches a chartsheet, e.g. `target`
/// `"chartsheets/sheet1.xml"`.
#[must_use]
pub fn workbook_chartsheet_relationship(id: &str, target: &str) -> String {
    format!(
        r#"<Relationship Id="{}" Type="{CHARTSHEET_RELATIONSHIP_TYPE}" Target="{}"/>"#,
        escape(id),
        escape(target),
    )
}

/// The `<sheet>` element that lists a chartsheet (or any sheet) in
/// `xl/workbook.xml`, for the workbook relationship `relationship_id`.
///
/// Excel refuses a sheet name that is empty, longer than 31 characters,
/// holds any of `\ / ? * [ ] :`, or starts or ends with an apostrophe, so such
/// a name is refused here rather than written. Two sheets whose names differ
/// only by case clash in Excel too; keeping names unique is yours.
pub fn workbook_sheet_element(
    name: &str,
    sheet_id: u32,
    relationship_id: &str,
) -> Result<String, ChartError> {
    let bad = name.is_empty()
        || name.chars().count() > 31
        || name.chars().any(|c| r"\/?*[]:".contains(c))
        || name.starts_with('\'')
        || name.ends_with('\'');
    if bad {
        return Err(ChartError::Unsupported {
            what: format!("the sheet name {name:?}"),
        });
    }
    Ok(format!(
        r#"<sheet name="{}" sheetId="{sheet_id}" r:id="{}"/>"#,
        escape(name),
        escape(relationship_id),
    ))
}
