//! Random specs, built from hostile and ordinary values alike.
//!
//! `render` takes data that may come from JSON or a user, so it must never
//! panic, and whatever it does accept must be well-formed XML with no
//! character XML 1.0 forbids. Each run is deterministic: the generator is a
//! fixed-seed linear congruential one, so a failure reproduces.

use ooxml_chart::{
    Axis, ChartKind, ChartSpec, DataLabelPosition, DataLabels, DateUnit, DisplayUnit, Effects,
    ErrorAmount, ErrorBars, Glow, LegendPosition, MarkerSymbol, OfPie, OfPieSplit, Plot, Series,
    SeriesName, Shadow, View3D,
};

struct Rng {
    state: u64,
    /// Only ordinary values, so that many specs get past validation.
    tame: bool,
}

impl Rng {
    fn next(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.state >> 33
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn chance(&mut self, one_in: usize) -> bool {
        self.below(one_in) == 0
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }

    fn number(&mut self) -> f64 {
        if self.tame {
            return *self.pick(&[1.0, 0.5, 100.0, 7.25]);
        }
        *self.pick(&[
            0.0,
            -0.0,
            1.0,
            -1.0,
            0.5,
            100.0,
            1e-9,
            1e300,
            -1e300,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::MAX,
            f64::MIN_POSITIVE,
        ])
    }

    fn color(&mut self) -> String {
        if self.tame {
            "8E0DD1".to_string()
        } else {
            self.text()
        }
    }

    fn text(&mut self) -> String {
        if self.tame {
            return self.pick(&["plain", "a & b < c", "Revenue"]).to_string();
        }
        let base: &str = self.pick(&[
            "",
            "plain",
            "a & b < c > d \" e ' f",
            "tab\there\nnewline\r\n",
            "nul\0inside",
            "\u{FFFE}\u{FFFF}",
            "emoji \u{1F600} and \u{E000}",
            "]]>",
            "<c:evil/>",
            "Sheet1!$A$1:$B$2",
            "'Sheet1'!$A$1:$A$9",
            "'It''s'!$C$3:$C$40",
            "not a reference",
            "#FFFFFF",
            "8E0DD1",
            "zzzzzz",
        ]);
        let mut text = base.to_string();
        if self.chance(20) {
            text.push_str(&"x".repeat(5000));
        }
        text
    }

    fn reference(&mut self) -> String {
        if self.tame {
            return "'Sheet1'!$B$2:$B$5".to_string();
        }
        self.pick(&[
            "'Sheet1'!$B$2:$B$5".to_string(),
            "Sheet1!$A$1:$C$9".to_string(),
            "S!$A$1".to_string(),
            String::new(),
            "no bang".to_string(),
        ])
        .clone()
    }
}

const KINDS: [ChartKind; 26] = [
    ChartKind::BarClustered,
    ChartKind::BarStacked,
    ChartKind::ColumnClustered,
    ChartKind::ColumnStacked,
    ChartKind::Line,
    ChartKind::LineMarkers,
    ChartKind::Pie,
    ChartKind::BarPercentStacked,
    ChartKind::ColumnPercentStacked,
    ChartKind::Area,
    ChartKind::AreaStacked,
    ChartKind::AreaPercentStacked,
    ChartKind::Scatter,
    ChartKind::ScatterLines,
    ChartKind::Doughnut,
    ChartKind::Radar,
    ChartKind::RadarFilled,
    ChartKind::Bubble,
    ChartKind::StockHighLowClose,
    ChartKind::StockOpenHighLowClose,
    ChartKind::PieOfPie,
    ChartKind::BarOfPie,
    ChartKind::Surface,
    ChartKind::SurfaceWireframe,
    ChartKind::Contour,
    ChartKind::ContourWireframe,
];

fn series(rng: &mut Rng) -> Series {
    let name = if rng.chance(2) {
        SeriesName::Literal(rng.text())
    } else {
        SeriesName::Reference(rng.reference())
    };
    let mut item = Series::new(name, rng.reference());
    if rng.chance(2) {
        item = item.with_categories(rng.reference());
    }
    if rng.chance(8) {
        item = item.with_multi_level_categories(rng.reference());
    }
    if rng.chance(4) {
        item = item.with_color(rng.color());
    }
    if rng.chance(6) {
        item = item.with_line_width(rng.number());
    }
    if rng.chance(6) {
        item = item.with_marker(MarkerSymbol::Circle, rng.below(300) as u8);
    }
    if rng.chance(6) {
        item = item.with_smooth(true);
    }
    if rng.chance(5) {
        item = item.with_cached_values((0..rng.below(6)).map(|_| rng.number()).collect());
    }
    if rng.chance(5) {
        item = item.with_cached_categories((0..rng.below(6)).map(|_| rng.text()).collect());
    }
    if rng.chance(8) {
        item = item.with_cached_name(rng.text());
    }
    if rng.chance(5) {
        item = item.with_bubble_sizes(rng.reference());
    }
    if rng.chance(8) {
        item = item.with_label_range(rng.reference());
    }
    if rng.chance(8) {
        item = item.with_error_bars(ErrorBars::new(ErrorAmount::Fixed(rng.number())));
    }
    if rng.chance(8) {
        let shadow = Shadow::new(rng.color())
            .blur(rng.number())
            .angle(rng.below(400) as u16)
            .opacity(rng.below(120) as u8);
        item = item.with_effects(
            Effects::new()
                .shadow(shadow)
                .glow(Glow::new(rng.color(), rng.number())),
        );
    }
    item
}

fn axis(rng: &mut Rng) -> Axis {
    let mut axis = Axis::default();
    if rng.chance(3) {
        axis = axis.title(rng.text());
    }
    if rng.chance(4) {
        axis = axis.number_format(rng.text());
    }
    if rng.chance(4) {
        axis = axis.min(rng.number());
    }
    if rng.chance(4) {
        axis = axis.max(rng.number());
    }
    if rng.chance(6) {
        axis = axis.major_unit(rng.number());
    }
    if rng.chance(8) {
        axis = axis.log(rng.below(20) as u16);
    }
    if rng.chance(8) {
        axis = axis.label_rotation(rng.below(800) as i16 - 400);
    }
    if rng.chance(6) {
        axis = axis.display_units(if rng.chance(2) {
            DisplayUnit::Thousands
        } else {
            DisplayUnit::Custom(rng.number())
        });
    }
    if rng.chance(8) {
        axis = axis.display_units_caption(rng.text());
    }
    if rng.chance(8) {
        axis = axis.crosses_at(rng.number());
    }
    if rng.chance(8) {
        axis = axis.dates(DateUnit::Months);
    }
    axis
}

fn spec(rng: &mut Rng) -> ChartSpec {
    let kind = *rng.pick(&KINDS);
    let mut spec = ChartSpec::new(kind);
    for _ in 0..rng.below(4) {
        spec = spec.series(series(rng));
    }
    if rng.chance(3) {
        spec = spec.title(rng.text());
    }
    if rng.chance(4) {
        spec = spec.legend(LegendPosition::Bottom);
    }
    if rng.chance(3) {
        spec = spec.category_axis(axis(rng));
    }
    if rng.chance(3) {
        spec = spec.value_axis(axis(rng));
    }
    if rng.chance(6) {
        spec = spec.gap_width(rng.below(700) as u16);
    }
    if rng.chance(6) {
        spec = spec.overlap(rng.below(300) as i16 - 150);
    }
    if rng.chance(5) {
        spec = spec.data_labels(DataLabels::values().at(DataLabelPosition::Above));
    }
    if rng.chance(5) {
        let mut plot = Plot::new(*rng.pick(&KINDS));
        for _ in 0..rng.below(3) {
            plot = plot.series(series(rng));
        }
        if rng.chance(2) {
            plot = plot.on_secondary_axis();
        }
        spec = spec.plot(plot);
    }
    if rng.chance(5) {
        spec = spec.secondary_value_axis(axis(rng));
    }
    if rng.chance(6) {
        spec = spec.view_3d(View3D::new().rotation_x(rng.below(300) as i16 - 150));
    }
    if rng.chance(8) {
        spec = spec.of_pie(OfPie::new().split(OfPieSplit::LastPoints(rng.below(10) as u32)));
    }
    if rng.chance(8) {
        spec = spec.surface_bands([rng.color(), rng.color()]);
    }
    if rng.chance(8) {
        spec = spec.stock_bars(rng.color(), rng.color());
    }
    if rng.chance(8) {
        spec = spec.style(rng.below(100) as u8).language(rng.text());
    }
    if rng.chance(8) {
        spec = spec.drop_lines(true).high_low_lines(rng.chance(2));
    }
    if rng.chance(8) {
        spec = spec.series_lines(true);
    }
    if rng.chance(8) {
        spec = spec.hole_size(rng.below(120) as u8);
    }
    spec
}

/// A tag-stack check, plus the characters XML 1.0 forbids.
fn assert_clean(xml: &str, context: &str) {
    for c in xml.chars() {
        let legal = matches!(
            c,
            '\t' | '\n' | '\r' | '\u{20}'..='\u{D7FF}' | '\u{E000}'..='\u{FFFD}' | '\u{10000}'..='\u{10FFFF}'
        );
        assert!(legal, "illegal character {c:?} in output for {context}");
    }
    let mut stack: Vec<&str> = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find('<') {
        rest = &rest[start + 1..];
        let end = rest.find('>').expect("unterminated tag");
        let tag = &rest[..end];
        rest = &rest[end + 1..];
        assert!(!rest.starts_with("]]>"), "stray CDATA end for {context}");
        if tag.starts_with('?') {
            continue;
        }
        if let Some(name) = tag.strip_prefix('/') {
            assert_eq!(stack.pop(), Some(name), "mismatched close for {context}");
        } else if !tag.ends_with('/') {
            stack.push(tag.split_whitespace().next().expect("a tag name"));
        }
    }
    assert!(stack.is_empty(), "unclosed {stack:?} for {context}");
}

#[test]
fn random_specs_never_panic_and_never_write_broken_xml() {
    let mut rng = Rng {
        state: 0x00C0_FFEE,
        tame: false,
    };
    let mut accepted = 0;
    for round in 0..40_000 {
        rng.tame = rng.chance(2);
        let spec = spec(&mut rng);
        if let Ok(part) = spec.render() {
            accepted += 1;
            assert_clean(
                &String::from_utf8_lossy(&part.xml),
                &format!("round {round}: {spec:?}"),
            );
        }
    }
    // The generator must reach valid specs too, or the test proves little.
    assert!(accepted > 1000, "only {accepted} specs were accepted");
}

#[test]
fn a_damaged_template_is_refused_or_read_but_never_panics() {
    let mut rng = Rng {
        state: 0xBAD_5EED,
        tame: true,
    };
    let mut parts = Vec::new();
    for kind in KINDS {
        let mut spec = ChartSpec::new(kind).title("Template");
        for _ in 0..2 {
            spec = spec.series(series(&mut rng));
        }
        if let Ok(part) = spec.render() {
            parts.push(part.xml);
        }
    }
    assert!(parts.len() > 5, "too few templates to damage");
    for _ in 0..20_000 {
        let mut bytes = parts[rng.below(parts.len())].clone();
        match rng.below(4) {
            0 => bytes.truncate(rng.below(bytes.len() + 1)),
            1 => {
                for _ in 0..=rng.below(8) {
                    let at = rng.below(bytes.len());
                    bytes[at] = rng.below(256) as u8;
                }
            }
            2 => {
                let from = rng.below(bytes.len());
                let to = (from + rng.below(200)).min(bytes.len());
                bytes.drain(from..to);
            }
            _ => {
                let at = rng.below(bytes.len());
                bytes.insert(at, b"<>&\"'/"[rng.below(6)]);
            }
        }
        // Either answer is fine; a panic is not.
        let _ = ChartSpec::from_template(&bytes);
    }
}
