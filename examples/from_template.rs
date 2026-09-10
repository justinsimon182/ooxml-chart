//! Adding a chart to a family that already exists.
//!
//! Every colour, font and axis setting is inherited from a chart that is known
//! to render correctly; only the data references and the title change.
//!
//! Run with `cargo run -p ooxml-chart --example from_template`.

use ooxml_chart::ChartSpec;

fn main() {
    // In a real program this is the bytes of an existing `xl/charts/chartN.xml`
    // read out of the package you are extending.
    let sibling: &[u8] = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><c:chart><c:title><c:tx><c:rich><a:p><a:r><a:t>Revenue by region</a:t></a:r></a:p></c:rich></c:tx></c:title><c:plotArea><c:barChart><c:ser><c:cat><c:strRef><c:f>Sheet1!$A$3:$A$38</c:f></c:strRef></c:cat><c:val><c:numRef><c:f>Sheet1!$C$3:$C$38</c:f></c:numRef></c:val></c:ser></c:barChart></c:plotArea></c:chart></c:chartSpace>"#;

    let template = ChartSpec::from_template(sibling).expect("a chart part");
    println!(
        "this template wants {} references",
        template.reference_count()
    );

    let part = template
        .with_references(vec![
            "Sheet1!$A$3:$A$38".to_string(),
            "Sheet1!$J$3:$J$38".to_string(),
        ])
        .with_title("Revenue by region (West)")
        .render()
        .expect("a chart");

    println!("{}", String::from_utf8_lossy(&part.xml));
}
