//! Original-function point sidecar comparison. This tests shared numeric/input
//! semantics, not React component identity, SpringValues or line path drawing.
use kartis::*;

fn input() -> Vec<BumpSeries> {
    vec![
        BumpSeries::new(
            "alpha",
            vec![
                BumpDatum::gap("Q3"),
                BumpDatum::new("Q1", 0.),
                BumpDatum::new(1., 3.),
                BumpDatum::new("1", -1.),
                BumpDatum::gap("Q4"),
            ],
        ),
        BumpSeries::new(
            "beta",
            vec![
                BumpDatum::new("Q4", 2.),
                BumpDatum::gap("Q3"),
                BumpDatum::new("Q1", 3.),
            ],
        ),
        BumpSeries::new("gap-only", vec![BumpDatum::gap("Q3"), BumpDatum::gap("Q4")]),
    ]
}
fn layout(size: f64) -> BumpLayout {
    let c = ChartConfig {
        width: 240.,
        height: 160.,
        margin: Margin {
            left: 0.,
            right: 0.,
            top: 0.,
            bottom: 0.,
        },
        ..Default::default()
    };
    let mut o = BumpLayoutOptions {
        axes: BumpAxes::none(),
        legend: false,
        labels: BumpEndpointLabels {
            start: false,
            end: false,
            ..Default::default()
        },
        ..Default::default()
    };
    o.style.point.size = size;
    let built = bump_layout(&c, &input(), &o).unwrap();
    // The adapter must not make the previously public layout literal private.
    BumpLayout {
        scene: built.scene,
        plot: built.plot,
        series: built.series,
        x_scale: built.x_scale,
        rank_scale: built.rank_scale,
        interaction: built.interaction,
    }
}
fn native_rows() -> Vec<Vec<&'static str>> {
    include_str!("fixtures/bump-point-widgets/points.tsv")
        .lines()
        .skip(1)
        .map(|line| line.split('\t').collect())
        .collect()
}

#[test]
fn original_compute_series_point_coordinates_and_uncompressed_indices_match() {
    let l = layout(6.);
    let rows = native_rows();
    assert_eq!(rows.len(), 5);
    assert_eq!(
        l.series.iter().map(|s| s.points.len()).sum::<usize>(),
        10,
        "Rust computed metadata also keeps gap rows"
    );
    for row in rows {
        assert_eq!(row.len(), 7);
        let series = l.series.iter().find(|s| s.source.id == row[0]).unwrap();
        let index = row[1].parse::<usize>().unwrap();
        let p = &series.points[index];
        assert_eq!(p.source_index, index);
        let x = match row[2] {
            "string" => CategoryKey::String(row[3].into()),
            "number" => CategoryKey::Number(row[3].parse().unwrap()),
            _ => panic!("unknown original key variant"),
        };
        assert_eq!(p.raw_x, x);
        assert_eq!(p.rank, Some(row[4].parse().unwrap()));
        let position = p.position.unwrap();
        for (actual, expected) in position.into_iter().zip([row[5], row[6]]) {
            let expected = expected.parse::<f64>().unwrap();
            assert!(
                (actual - expected).abs() <= 1e-12,
                "{actual} != original {expected}"
            );
        }
    }
    assert_ne!(
        l.series[0].points[2].position, l.series[0].points[3].position,
        "numeric1 and string1 keep distinct category positions"
    );
    assert!(
        l.series[2]
            .points
            .iter()
            .all(|p| p.position.is_none() && p.mark_id.is_none())
    );
}

#[test]
fn native_non_null_creation_order_survives_zero_size_without_inventing_gap_components() {
    let expected = native_rows()
        .iter()
        .map(|r| format!("{}.{}", r[0], r[1]))
        .collect::<Vec<_>>();
    for size in [6., 0.] {
        let l = layout(size);
        let mut calls = vec![];
        let packet = bump_point_widgets(&l, 7, |c| {
            assert!(c.datum.rank.is_some());
            assert_eq!(c.datum, &c.series.data[c.point.source_index]);
            assert!(c.point.mark_id.is_some());
            assert!(c.point.position.is_some());
            calls.push(format!("{}.{}", c.series.id, c.point.source_index));
            Ok(Some(PlotWidgetPlacement::AtDatum {
                offset: [0., 0.],
                measured: TextSize {
                    width: 1.,
                    height: 1.,
                },
            }))
        })
        .unwrap();
        assert_eq!(calls, expected);
        assert_eq!(packet.data.len(), 5);
        assert!(packet.layout.slots.iter().all(|slot| slot.revision == 7));
        assert!(packet.data.values().any(|d| d.point.rank == Some(0.)));
        assert!(packet.data.values().all(|d| d.point.rank.is_some()));
        let alpha = packet
            .data
            .values()
            .find(|d| d.series.id == "alpha")
            .unwrap();
        assert_eq!(alpha.series.data.len(), 5);
        assert!(alpha.series.data[0].rank.is_none());
        assert!(alpha.series.data[4].rank.is_none());
        for d in packet.data.values() {
            let mark = l
                .scene
                .marks
                .iter()
                .find(|m| Some(&m.id) == d.point.mark_id.as_ref())
                .unwrap();
            assert!(matches!(mark.geometry,Geometry::Circle{radius,..}if radius==size/2.));
        }
    }
}
