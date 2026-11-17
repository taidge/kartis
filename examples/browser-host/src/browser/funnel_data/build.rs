use super::*;

fn callback(
    m: &Metrics,
    kind: &str,
    source_index: usize,
    d: &Raw,
    all: &[FunnelDataDatum<Raw>],
    computed: Value,
) {
    log(
        m,
        kind,
        json!({"sourceIndex":source_index,"raw":raw(d),"source":all.iter().map(|d|raw(&d.data)).collect::<Vec<_>>(),"computed":computed}),
    );
}
pub(super) fn build(m: &Metrics, ctx: &BrowserFunnelDataBuildContext<'_, Raw>) -> Result<Frame> {
    m.builds.set(m.builds.get() + 1);
    log(
        m,
        "build",
        json!({"source":ctx.source.iter().map(|d|raw(&d.data)).collect::<Vec<_>>(),"current":ctx.options.current.as_ref().map(super::super::funnel_svg_layers::key_json)}),
    );
    let mut color = |c: &FunnelDataColorContext<'_, Raw>| {
        callback(
            m,
            "color",
            c.computed.source_index,
            c.data,
            c.source,
            json!({"id":super::super::funnel_svg_layers::key_json(&c.computed.datum.id),"valueBits":c.computed.datum.value.to_bits().to_string()}),
        );
        if m.fault.get() == 1 {
            return Err(ChartError("raw color rejected".into()));
        }
        Ok(c.computed.default_color)
    };
    let mut border = |c: &FunnelDataStyleContext<'_, Raw>| {
        callback(
            m,
            "border",
            c.computed.part.source_index,
            c.data,
            c.source,
            json!({"id":super::super::funnel_svg_layers::key_json(&c.computed.part.datum.id),"valueBits":c.computed.part.datum.value.to_bits().to_string(),"center":c.computed.part.center}),
        );
        Ok(c.computed.default_color)
    };
    let mut label = |c: &FunnelDataStyleContext<'_, Raw>| {
        callback(
            m,
            "label",
            c.computed.part.source_index,
            c.data,
            c.source,
            json!({"id":super::super::funnel_svg_layers::key_json(&c.computed.part.datum.id),"valueBits":c.computed.part.datum.value.to_bits().to_string(),"center":c.computed.part.center}),
        );
        Ok(c.computed.default_color)
    };
    let mut size = |c: FunnelDataSizeContext<'_, Raw>| {
        callback(
            m,
            "size",
            c.computed.source_index,
            c.data,
            c.source,
            json!({"id":super::super::funnel_svg_layers::key_json(&c.computed.datum.id),"valueBits":c.computed.datum.value.to_bits().to_string(),"scaled":c.computed.scaled_size,"available":c.computed.available_size}),
        );
        Ok(c.computed.datum.size.unwrap_or(0.))
    };
    let mut stage = |c: &FunnelDataStageContext<'_, Raw>| {
        callback(
            m,
            "stage",
            c.computed.source_index,
            c.data,
            c.source,
            json!({"id":super::super::funnel_svg_layers::key_json(&c.computed.datum.id),"valueBits":c.computed.datum.value.to_bits().to_string(),"center":c.computed.part.center}),
        );
        if m.fault.get() == 2 {
            return Err(ChartError("raw stage rejected".into()));
        }
        Ok(c.computed.inherit(c.computed.geometry.clone()))
    };
    let mut format = |v: f64| {
        log(m, "format", json!({"bits":v.to_bits().to_string()}));
        Ok(format!("value:{v}"))
    };
    let base = Frame::with_shared_factories(
        ctx.config,
        ctx.source.clone(),
        ctx.options,
        &mut FunnelDataFactories {
            color: Some(&mut color),
            formatted_value: Some(&mut format),
            border_color: Some(&mut border),
            label_color: Some(&mut label),
            size: Some(&mut size),
            stage: Some(&mut stage),
        },
    )?;
    let mut matcher = |c: FunnelDataAnnotationMatchContext<'_, Raw>| {
        log(
            m,
            "match",
            json!({"raw":raw(c.data),"base":data(c.frame),"sourceIndex":c.computed.source_index,
            "partIndex":c.computed.part_index,"collection":c.computed.collection.iter().map(|p|p.source_index).collect::<Vec<_>>()}),
        );
        if m.fault.get() == 3 {
            return Err(ChartError("raw predicate rejected".into()));
        }
        Ok(c.data.region == "region-3")
    };
    let mut note = |c: FunnelDataAnnotationNoteContext<'_, Raw>| {
        log(
            m,
            "note",
            json!({"raw":raw(c.data),"base":data(c.frame),"sourceIndex":c.computed.binding.source_index,
            "partIndex":c.computed.binding.part_index,"position":[c.computed.x,c.computed.y]}),
        );
        if m.fault.get() == 4 {
            return Err(ChartError("raw note rejected".into()));
        }
        let mut scene = c.scene()?;
        scene.marks.push(Mark::new(
            "raw-note",
            Geometry::Rect {
                x: c.computed.x,
                y: c.computed.y - 10.,
                width: 14.,
                height: 12.,
            },
            Color(20, 180, 210, 255),
        ));
        Ok(scene)
    };
    let mut spec = FunnelAnnotationSpec::new(
        FunnelAnnotationMatch::Predicate("raw-note".into()),
        FunnelAnnotationKind::Circle,
        FunnelAnnotationNote::Custom("raw-note".into()),
        FunnelAnnotationPosition::Relative(-80.),
        FunnelAnnotationPosition::Relative(-16.),
    );
    spec.size = Some(20.);
    spec.note_width = 32.;
    let mut budget = FunnelAnnotationBudget::default();
    if m.fault.get() == 5 {
        budget.bindings = 0;
    }
    let value = base.with_annotations_with_budget(
        &[spec],
        &mut FunnelDataAnnotationFactories {
            matcher: Some(&mut matcher),
            note: Some(&mut note),
        },
        &FunnelAnnotationStyle::default(),
        budget,
    )?;
    if m.fault.get() == 6 {
        // Same projection, foreign raw Arc: the owner must reject this candidate
        // before any retained layer or tooltip can observe it.
        return Frame::new(ctx.config, source(999, 0), ctx.options);
    }
    Ok(value)
}
