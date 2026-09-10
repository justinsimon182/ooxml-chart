//! The drawing part: where each chart sits, and which chart it is.
//!
//! # Why the relationship ids are yours to supply
//!
//! It is tempting to number a drawing's relationships `rId1`, `rId2`, … as they
//! are written. Some workbooks do not: one real corpus numbers them after the
//! chart part, so `chart36.xml` is reached by `rId36`. A drawing whose ids do
//! not match what the rest of the package expects still opens — it just shows
//! the wrong charts, which is the kind of wrong that looks right. So this takes
//! the ids rather than inventing them.

use crate::metrics::{CellAnchor, TwoCellAnchor};
use crate::xml::escape;

/// The relationship type a drawing uses to reach a chart.
pub const CHART_RELATIONSHIP_TYPE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart";

/// The frame that hosts one chart inside a drawing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphicFrame {
    /// The frame's document-unique id. Excel's own numbering starts at 2.
    pub id: u32,
    /// The frame's name, e.g. `"Chart 1"`. Shown in Excel's selection pane.
    pub name: String,
    /// The relationship on the drawing that reaches this frame's chart part.
    pub relationship_id: String,
    /// The anchor's `editAs`, or `None` to omit the attribute.
    ///
    /// `editAs` decides what happens to the chart when the rows and columns
    /// underneath it are resized: `"twoCell"` moves *and* sizes it,
    /// `"oneCell"` moves it without resizing, `"absolute"` does neither.
    /// Omitting the attribute means `"twoCell"`, which is both the schema's
    /// default and what real drawings overwhelmingly carry — a corpus
    /// `drawing1.xml` audited for this holds seven `<xdr:twoCellAnchor>`
    /// elements and not one `editAs` among them.
    ///
    /// So `None` is the default here, and it matters: a chart added to a
    /// drawing whose other anchors omit the attribute, but which carries one
    /// itself, resizes differently from every chart beside it. The workbook
    /// opens cleanly and nothing looks wrong until someone changes a row
    /// height.
    pub edit_as: Option<String>,
}

/// One `<xdr:twoCellAnchor>` hosting one chart.
#[must_use]
pub fn anchor_xml(anchor: &TwoCellAnchor, frame: &GraphicFrame) -> String {
    format!(
        concat!(
            "<xdr:twoCellAnchor{edit_as}>",
            "{from}{to}",
            r#"<xdr:graphicFrame macro="">"#,
            r#"<xdr:nvGraphicFramePr><xdr:cNvPr id="{id}" name="{name}"/><xdr:cNvGraphicFramePr/></xdr:nvGraphicFramePr>"#,
            r#"<xdr:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/></xdr:xfrm>"#,
            r#"<a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">"#,
            r#"<c:chart xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart""#,
            r#" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships""#,
            r#" r:id="{rid}"/>"#,
            r#"</a:graphicData></a:graphic></xdr:graphicFrame><xdr:clientData/></xdr:twoCellAnchor>"#,
        ),
        edit_as = match &frame.edit_as {
            Some(value) => format!(r#" editAs="{}""#, escape(value)),
            None => String::new(),
        },
        from = corner_xml("from", &anchor.from),
        to = corner_xml("to", &anchor.to),
        id = frame.id,
        name = escape(&frame.name),
        rid = escape(&frame.relationship_id),
    )
}

fn corner_xml(tag: &str, corner: &CellAnchor) -> String {
    format!(
        "<xdr:{tag}><xdr:col>{col}</xdr:col><xdr:colOff>{col_off}</xdr:colOff><xdr:row>{row}</xdr:row><xdr:rowOff>{row_off}</xdr:rowOff></xdr:{tag}>",
        col = corner.col,
        col_off = corner.col_offset_emu,
        row = corner.row,
        row_off = corner.row_offset_emu,
    )
}

/// A complete drawing part hosting every anchor given, in order.
#[must_use]
pub fn drawing_part(anchors: &[(TwoCellAnchor, GraphicFrame)]) -> Vec<u8> {
    let mut out = String::with_capacity(512 + anchors.len() * 768);
    out.push_str(concat!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
        r#"<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing""#,
        r#" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main""#,
        r#" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#,
    ));
    for (anchor, frame) in anchors {
        out.push_str(&anchor_xml(anchor, frame));
    }
    out.push_str("</xdr:wsDr>");
    out.into_bytes()
}

/// A drawing's `_rels` part: one entry per chart, with the ids given.
#[must_use]
pub fn drawing_relationships(chart_targets: &[(String, String)]) -> Vec<u8> {
    let mut out = String::with_capacity(256 + chart_targets.len() * 160);
    out.push_str(concat!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
    ));
    for (id, target) in chart_targets {
        out.push_str(&format!(
            r#"<Relationship Id="{}" Type="{CHART_RELATIONSHIP_TYPE}" Target="{}"/>"#,
            escape(id),
            escape(target),
        ));
    }
    out.push_str("</Relationships>");
    out.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anchor() -> TwoCellAnchor {
        TwoCellAnchor {
            from: CellAnchor {
                col: 10,
                col_offset_emu: 0,
                row: 1,
                row_offset_emu: 0,
            },
            to: CellAnchor {
                col: 20,
                col_offset_emu: 4762,
                row: 20,
                row_offset_emu: 9525,
            },
        }
    }

    fn frame() -> GraphicFrame {
        GraphicFrame {
            id: 2,
            name: "Chart 1".to_string(),
            relationship_id: "rId1".to_string(),
            edit_as: None,
        }
    }

    #[test]
    fn both_corners_are_written_as_column_offset_row_offset() {
        let out = anchor_xml(&anchor(), &frame());
        assert!(
            out.contains("<xdr:from><xdr:col>10</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>1</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:from>"),
            "{out}"
        );
        assert!(
            out.contains("<xdr:to><xdr:col>20</xdr:col><xdr:colOff>4762</xdr:colOff><xdr:row>20</xdr:row><xdr:rowOff>9525</xdr:rowOff></xdr:to>"),
            "{out}"
        );
    }

    #[test]
    fn the_frame_carries_the_relationship_that_reaches_the_chart() {
        let out = anchor_xml(&anchor(), &frame());
        assert!(out.contains(r#"r:id="rId1""#), "{out}");
        assert!(
            out.contains(r#"<xdr:cNvPr id="2" name="Chart 1"/>"#),
            "{out}"
        );
    }

    #[test]
    fn no_edit_as_is_written_unless_one_is_asked_for() {
        // Omitted means `twoCell` — move *and* size — which is what every
        // anchor in the audited corpus drawing carries. A chart that writes
        // the attribute when its siblings do not resizes differently from
        // them, in a workbook that opens perfectly.
        let out = anchor_xml(&anchor(), &frame());
        assert!(out.starts_with("<xdr:twoCellAnchor>"), "{out}");
        assert!(!out.contains("editAs"), "{out}");
    }

    #[test]
    fn an_edit_as_is_written_when_one_is_asked_for() {
        let pinned = GraphicFrame {
            edit_as: Some("oneCell".to_string()),
            ..frame()
        };
        let out = anchor_xml(&anchor(), &pinned);
        assert!(
            out.starts_with(r#"<xdr:twoCellAnchor editAs="oneCell">"#),
            "{out}"
        );
    }

    #[test]
    fn a_frame_name_is_escaped() {
        let named = GraphicFrame {
            name: "R&D <chart>".to_string(),
            ..frame()
        };
        let out = anchor_xml(&anchor(), &named);
        assert!(out.contains(r#"name="R&amp;D &lt;chart&gt;""#), "{out}");
    }

    #[test]
    fn a_drawing_part_wraps_every_anchor_in_one_document() {
        let part = drawing_part(&[(anchor(), frame()), (anchor(), frame())]);
        let out = String::from_utf8(part).expect("UTF-8");
        assert!(out.starts_with(r#"<?xml version="1.0""#), "{out}");
        assert!(out.contains("<xdr:wsDr"), "{out}");
        assert_eq!(out.matches("<xdr:twoCellAnchor").count(), 2, "{out}");
        assert!(out.ends_with("</xdr:wsDr>"), "{out}");
    }

    #[test]
    fn an_empty_drawing_is_still_a_valid_document() {
        let out = String::from_utf8(drawing_part(&[])).expect("UTF-8");
        assert!(out.contains("<xdr:wsDr"), "{out}");
        assert!(!out.contains("<xdr:twoCellAnchor"), "{out}");
    }

    #[test]
    fn the_relationships_part_lists_one_entry_per_chart_with_the_ids_given() {
        let out = String::from_utf8(drawing_relationships(&[
            ("rId36".to_string(), "../charts/chart36.xml".to_string()),
            ("rId37".to_string(), "../charts/chart37.xml".to_string()),
        ]))
        .expect("UTF-8");
        assert!(out.contains(r#"Id="rId36""#), "{out}");
        assert!(out.contains(r#"Target="../charts/chart36.xml""#), "{out}");
        assert!(out.contains(r#"Id="rId37""#), "{out}");
        assert_eq!(out.matches("<Relationship ").count(), 2, "{out}");
        assert!(out.contains(CHART_RELATIONSHIP_TYPE), "{out}");
    }
}
