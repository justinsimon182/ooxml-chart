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

/// The content type a drawing part declares in `[Content_Types].xml`.
pub const DRAWING_CONTENT_TYPE: &str = "application/vnd.openxmlformats-officedocument.drawing+xml";

/// The relationship type a worksheet uses to reach its drawing.
pub const DRAWING_RELATIONSHIP_TYPE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing";

/// The `[Content_Types].xml` entry for a drawing part, ready to insert before
/// `</Types>`. `part_name` is the package path with a leading slash, e.g.
/// `"/xl/drawings/drawing1.xml"`.
#[must_use]
pub fn drawing_content_types_override(part_name: &str) -> String {
    format!(
        r#"<Override PartName="{}" ContentType="{DRAWING_CONTENT_TYPE}"/>"#,
        escape(part_name),
    )
}

/// The worksheet's `_rels` entry that reaches a drawing, e.g. `id` `"rId1"` and
/// `target` `"../drawings/drawing1.xml"`. Insert it before `</Relationships>`
/// in `xl/worksheets/_rels/sheetN.xml.rels`.
#[must_use]
pub fn worksheet_drawing_relationship(id: &str, target: &str) -> String {
    format!(
        r#"<Relationship Id="{}" Type="{DRAWING_RELATIONSHIP_TYPE}" Target="{}"/>"#,
        escape(id),
        escape(target),
    )
}

/// The `<drawing>` element a worksheet carries to point at its drawing, for
/// the relationship `id`. The worksheet needs the `r` prefix declared
/// (`xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"`)
/// and this element placed where the schema puts it: after the page setup and
/// header/footer elements, before `legacyDrawing` and `tableParts`.
#[must_use]
pub fn worksheet_drawing_element(id: &str) -> String {
    format!(r#"<drawing r:id="{}"/>"#, escape(id))
}

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

/// The frame and its trailing `<xdr:clientData/>`, shared by every anchor type.
fn frame_xml(frame: &GraphicFrame) -> String {
    format!(
        concat!(
            r#"<xdr:graphicFrame macro="">"#,
            r#"<xdr:nvGraphicFramePr><xdr:cNvPr id="{id}" name="{name}"/><xdr:cNvGraphicFramePr/></xdr:nvGraphicFramePr>"#,
            r#"<xdr:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/></xdr:xfrm>"#,
            r#"<a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">"#,
            r#"<c:chart xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart""#,
            r#" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships""#,
            r#" r:id="{rid}"/>"#,
            r#"</a:graphicData></a:graphic></xdr:graphicFrame><xdr:clientData/>"#,
        ),
        id = frame.id,
        name = escape(&frame.name),
        rid = escape(&frame.relationship_id),
    )
}

/// One `<xdr:twoCellAnchor>` hosting one chart.
#[must_use]
pub fn anchor_xml(anchor: &TwoCellAnchor, frame: &GraphicFrame) -> String {
    format!(
        "<xdr:twoCellAnchor{edit_as}>{from}{to}{frame}</xdr:twoCellAnchor>",
        edit_as = match &frame.edit_as {
            Some(value) => format!(r#" editAs="{}""#, escape(value)),
            None => String::new(),
        },
        from = corner_xml("from", &anchor.from),
        to = corner_xml("to", &anchor.to),
        frame = frame_xml(frame),
    )
}

/// Where a chart sits in a drawing.
///
/// Prefer [`Anchor::TwoCell`], which is what real workbooks carry and what
/// [`crate::two_cell_anchor`] computes. The others exist for a chart that must
/// keep an exact size however the cells beneath it are resized.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Anchor {
    /// Two cell corners; the size is the distance between them.
    TwoCell(TwoCellAnchor),
    /// One cell corner and a fixed size in EMU. Moves with its cell, never
    /// resizes. The schema gives this anchor no `editAs`, so
    /// [`GraphicFrame::edit_as`] is not written.
    OneCell {
        /// The top-left corner.
        from: CellAnchor,
        /// Width in EMU.
        width_emu: u64,
        /// Height in EMU.
        height_emu: u64,
    },
    /// A fixed position and size on the sheet, in EMU, whatever the cells do.
    Absolute {
        /// Distance from the sheet's left edge.
        x_emu: u64,
        /// Distance from the sheet's top edge.
        y_emu: u64,
        /// Width in EMU.
        width_emu: u64,
        /// Height in EMU.
        height_emu: u64,
    },
}

/// One anchor of any type hosting one chart.
#[must_use]
pub fn anchor_xml_for(anchor: &Anchor, frame: &GraphicFrame) -> String {
    match anchor {
        Anchor::TwoCell(two_cell) => anchor_xml(two_cell, frame),
        Anchor::OneCell {
            from,
            width_emu,
            height_emu,
        } => format!(
            r#"<xdr:oneCellAnchor>{from}<xdr:ext cx="{width_emu}" cy="{height_emu}"/>{frame}</xdr:oneCellAnchor>"#,
            from = corner_xml("from", from),
            frame = frame_xml(frame),
        ),
        Anchor::Absolute {
            x_emu,
            y_emu,
            width_emu,
            height_emu,
        } => format!(
            r#"<xdr:absoluteAnchor><xdr:pos x="{x_emu}" y="{y_emu}"/><xdr:ext cx="{width_emu}" cy="{height_emu}"/>{frame}</xdr:absoluteAnchor>"#,
            frame = frame_xml(frame),
        ),
    }
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

const DRAWING_OPEN: &str = concat!(
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
    r#"<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing""#,
    r#" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main""#,
    r#" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#,
);

/// A complete drawing part hosting every anchor given, in order.
#[must_use]
pub fn drawing_part(anchors: &[(TwoCellAnchor, GraphicFrame)]) -> Vec<u8> {
    let mut out = String::with_capacity(512 + anchors.len() * 768);
    out.push_str(DRAWING_OPEN);
    for (anchor, frame) in anchors {
        out.push_str(&anchor_xml(anchor, frame));
    }
    out.push_str("</xdr:wsDr>");
    out.into_bytes()
}

/// Like [`drawing_part`], for anchors of any [`Anchor`] type, mixed freely.
#[must_use]
pub fn drawing_part_with(anchors: &[(Anchor, GraphicFrame)]) -> Vec<u8> {
    let mut out = String::with_capacity(512 + anchors.len() * 768);
    out.push_str(DRAWING_OPEN);
    for (anchor, frame) in anchors {
        out.push_str(&anchor_xml_for(anchor, frame));
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

/// The relationship type a drawing uses to reach an image.
pub const IMAGE_RELATIONSHIP_TYPE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image";

/// A picture in a drawing. The image bytes are a package part of your own
/// (`/xl/media/image1.png`); this only points at it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Picture {
    /// The picture's document-unique id. Excel's own numbering starts at 2.
    pub id: u32,
    /// The picture's name, e.g. `"Picture 1"`.
    pub name: String,
    /// The relationship on the drawing that reaches the image part, of type
    /// [`IMAGE_RELATIONSHIP_TYPE`].
    pub relationship_id: String,
    /// Alternative text for screen readers, or `None`.
    pub description: Option<String>,
}

impl Picture {
    /// A picture reached by `relationship_id`, with no alternative text.
    pub fn new(id: u32, name: impl Into<String>, relationship_id: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            relationship_id: relationship_id.into(),
            description: None,
        }
    }

    /// Sets the alternative text.
    #[must_use]
    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = Some(text.into());
        self
    }
}

/// A text box in a drawing: a rectangle holding plain text.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct TextBox {
    /// The text box's document-unique id. Excel's own numbering starts at 2.
    pub id: u32,
    /// The text box's name, e.g. `"TextBox 1"`.
    pub name: String,
    /// The text. Each line becomes a paragraph.
    pub text: String,
}

impl TextBox {
    /// A text box holding `text`.
    pub fn new(id: u32, name: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            text: text.into(),
        }
    }
}

/// Anything a drawing anchor can host.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DrawingObject {
    /// A chart.
    Chart(GraphicFrame),
    /// A picture.
    Picture(Picture),
    /// A text box.
    TextBox(TextBox),
}

fn picture_xml(picture: &Picture) -> String {
    let description = match &picture.description {
        Some(text) => format!(r#" descr="{}""#, escape(text)),
        None => String::new(),
    };
    format!(
        concat!(
            r#"<xdr:pic><xdr:nvPicPr><xdr:cNvPr id="{id}" name="{name}"{descr}/>"#,
            r#"<xdr:cNvPicPr><a:picLocks noChangeAspect="1"/></xdr:cNvPicPr></xdr:nvPicPr>"#,
            r#"<xdr:blipFill><a:blip r:embed="{rid}"/><a:stretch><a:fillRect/></a:stretch></xdr:blipFill>"#,
            r#"<xdr:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/></a:xfrm>"#,
            r#"<a:prstGeom prst="rect"><a:avLst/></a:prstGeom></xdr:spPr></xdr:pic><xdr:clientData/>"#,
        ),
        id = picture.id,
        name = escape(&picture.name),
        descr = description,
        rid = escape(&picture.relationship_id),
    )
}

fn text_box_xml(text_box: &TextBox) -> String {
    let paragraphs: String = text_box
        .text
        .split('\n')
        .map(|line| {
            let line = line.trim_end_matches('\r');
            if line.is_empty() {
                "<a:p/>".to_string()
            } else {
                format!("<a:p><a:r><a:t>{}</a:t></a:r></a:p>", escape(line))
            }
        })
        .collect();
    format!(
        concat!(
            r#"<xdr:sp macro="" textlink=""><xdr:nvSpPr><xdr:cNvPr id="{id}" name="{name}"/>"#,
            r#"<xdr:cNvSpPr txBox="1"/></xdr:nvSpPr>"#,
            r#"<xdr:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/></a:xfrm>"#,
            r#"<a:prstGeom prst="rect"><a:avLst/></a:prstGeom></xdr:spPr>"#,
            r#"<xdr:txBody><a:bodyPr/><a:lstStyle/>{paragraphs}</xdr:txBody></xdr:sp><xdr:clientData/>"#,
        ),
        id = text_box.id,
        name = escape(&text_box.name),
        paragraphs = paragraphs,
    )
}

/// One anchor of any type hosting any [`DrawingObject`].
///
/// A chart is written exactly as [`anchor_xml_for`] writes it. A picture or text
/// box carries no `editAs`, which means `"twoCell"` on a two-cell anchor.
#[must_use]
pub fn drawing_object_xml(anchor: &Anchor, object: &DrawingObject) -> String {
    let payload = match object {
        DrawingObject::Chart(frame) => return anchor_xml_for(anchor, frame),
        DrawingObject::Picture(picture) => picture_xml(picture),
        DrawingObject::TextBox(text_box) => text_box_xml(text_box),
    };
    match anchor {
        Anchor::TwoCell(two_cell) => format!(
            "<xdr:twoCellAnchor>{}{}{payload}</xdr:twoCellAnchor>",
            corner_xml("from", &two_cell.from),
            corner_xml("to", &two_cell.to),
        ),
        Anchor::OneCell {
            from,
            width_emu,
            height_emu,
        } => format!(
            r#"<xdr:oneCellAnchor>{}<xdr:ext cx="{width_emu}" cy="{height_emu}"/>{payload}</xdr:oneCellAnchor>"#,
            corner_xml("from", from),
        ),
        Anchor::Absolute {
            x_emu,
            y_emu,
            width_emu,
            height_emu,
        } => format!(
            r#"<xdr:absoluteAnchor><xdr:pos x="{x_emu}" y="{y_emu}"/><xdr:ext cx="{width_emu}" cy="{height_emu}"/>{payload}</xdr:absoluteAnchor>"#
        ),
    }
}

/// A complete drawing part hosting charts, pictures and text boxes, mixed
/// freely, in order.
#[must_use]
pub fn drawing_part_objects(objects: &[(Anchor, DrawingObject)]) -> Vec<u8> {
    let mut out = String::with_capacity(512 + objects.len() * 768);
    out.push_str(DRAWING_OPEN);
    for (anchor, object) in objects {
        out.push_str(&drawing_object_xml(anchor, object));
    }
    out.push_str("</xdr:wsDr>");
    out.into_bytes()
}

/// A drawing's `_rels` part for targets of any kind: each entry is
/// `(id, relationship type, target)`, e.g. [`CHART_RELATIONSHIP_TYPE`] or
/// [`IMAGE_RELATIONSHIP_TYPE`].
#[must_use]
pub fn drawing_relationships_typed(entries: &[(String, &str, String)]) -> Vec<u8> {
    let mut out = String::with_capacity(256 + entries.len() * 160);
    out.push_str(concat!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
    ));
    for (id, kind, target) in entries {
        out.push_str(&format!(
            r#"<Relationship Id="{}" Type="{}" Target="{}"/>"#,
            escape(id),
            escape(kind),
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

    #[test]
    fn the_drawing_content_types_entry_names_the_part_and_type() {
        assert_eq!(
            drawing_content_types_override("/xl/drawings/drawing1.xml"),
            r#"<Override PartName="/xl/drawings/drawing1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawing+xml"/>"#
        );
    }

    #[test]
    fn the_worksheet_relationship_and_element_carry_the_id() {
        assert_eq!(
            worksheet_drawing_relationship("rId2", "../drawings/drawing1.xml"),
            r#"<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing" Target="../drawings/drawing1.xml"/>"#
        );
        assert_eq!(
            worksheet_drawing_element("rId2"),
            r#"<drawing r:id="rId2"/>"#
        );
    }

    #[test]
    fn names_and_ids_are_escaped() {
        assert!(drawing_content_types_override("/a&b.xml").contains("/a&amp;b.xml"));
        assert!(worksheet_drawing_relationship("r&1", "x<y").contains(r#"Id="r&amp;1""#));
        assert!(worksheet_drawing_element("r\"1").contains("r&quot;1"));
    }
}
