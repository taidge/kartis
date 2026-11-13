//! Bounded GeoJSON and TopoJSON ingestion for typed `GeoFeature` polygons.
//!
//! Nivo geo/src/hooks.js consumes GeoJSON features before d3-geo projection.
//! Topology decoding follows the TopoJSON specification: only transformed arcs
//! are delta encoded; a negative reference is one's-complement and reverses the
//! complete decoded arc. Stitching validates endpoints before removing duplicates.
use std::collections::BTreeSet;

use serde_json::{Map, Value};

use crate::scene::{MAX_LABEL_BYTES, ensure};
use crate::{ChartError, GeoFeature, Result};

pub const MAX_GEO_JSON_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_GEO_FEATURES: usize = 20_000;
pub const MAX_GEO_POINTS: usize = 100_000;
type Ring = Vec<[f64; 2]>;
type Polygon = Vec<Ring>;

#[derive(Clone, Debug, PartialEq)]
pub struct GeoParseOptions {
    /// `None` reads the standard feature/geometry `id` member.
    pub id_property: Option<String>,
    /// Missing labels fall back to the id; present labels must be strings.
    pub label_property: Option<String>,
    /// Optional numeric property used by the choropleth color scale.
    pub value_property: Option<String>,
    /// Missing/null values use this explicit fallback; never coerce strings.
    pub missing_value: Option<f64>,
    pub max_features: usize,
    pub max_points: usize,
}
impl Default for GeoParseOptions {
    fn default() -> Self {
        Self {
            id_property: None,
            label_property: Some("name".into()),
            value_property: None,
            missing_value: None,
            max_features: MAX_GEO_FEATURES,
            max_points: MAX_GEO_POINTS,
        }
    }
}
struct Budget {
    points: usize,
    features: usize,
    max_points: usize,
    max_features: usize,
}
impl Budget {
    fn new(o: &GeoParseOptions) -> Result<Self> {
        ensure(
            o.max_features <= MAX_GEO_FEATURES && o.max_points <= MAX_GEO_POINTS,
            "geodata limits exceed hard budgets",
        )?;
        ensure(
            o.missing_value.is_none_or(f64::is_finite),
            "missing geo value must be finite",
        )?;
        for p in [&o.id_property, &o.label_property, &o.value_property]
            .into_iter()
            .flatten()
        {
            ensure(!p.is_empty() && p.len() <= 256, "invalid geo property name")?;
        }
        Ok(Self {
            points: 0,
            features: 0,
            max_points: o.max_points,
            max_features: o.max_features,
        })
    }
    fn point(&mut self) -> Result<()> {
        self.points += 1;
        ensure(self.points <= self.max_points, "geo point budget exceeded")
    }
    fn feature(&mut self) -> Result<()> {
        self.features += 1;
        ensure(
            self.features <= self.max_features,
            "geo feature budget exceeded",
        )
    }
}
fn error(message: &str) -> ChartError {
    ChartError(message.into())
}
fn object(v: &Value) -> Result<&Map<String, Value>> {
    v.as_object().ok_or_else(|| error("expected geo object"))
}
fn array(v: &Value) -> Result<&Vec<Value>> {
    v.as_array()
        .ok_or_else(|| error("expected geo coordinate/arc array"))
}
fn field<'a>(v: &'a Value, name: &str) -> Result<&'a Value> {
    v.get(name)
        .ok_or_else(|| error(&format!("missing geo field '{name}'")))
}
fn kind(v: &Value) -> Result<&str> {
    field(v, "type")?
        .as_str()
        .ok_or_else(|| error("geo type must be a string"))
}
fn number(v: &Value) -> Result<f64> {
    let n = v
        .as_f64()
        .ok_or_else(|| error("geo number must be numeric"))?;
    ensure(n.is_finite(), "geo numbers must be finite")?;
    Ok(n)
}
fn parse(input: &str) -> Result<Value> {
    ensure(
        input.len() <= MAX_GEO_JSON_BYTES,
        "geo JSON byte budget exceeded",
    )?;
    serde_json::from_str(input).map_err(|e| ChartError(format!("invalid geo JSON: {e}")))
}
fn id_string(v: &Value) -> Result<String> {
    let id = match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => return Err(error("geo id must be a string or number")),
    };
    ensure(
        !id.trim().is_empty() && id.len() <= 1000,
        "geo id must be nonempty and at most 1000 bytes",
    )?;
    Ok(id)
}
fn metadata(v: &Value, o: &GeoParseOptions) -> Result<(String, String, Option<f64>)> {
    let properties = match v.get("properties") {
        None | Some(Value::Null) => None,
        Some(value) => Some(object(value)?),
    };
    let id = if let Some(property) = &o.id_property {
        properties
            .and_then(|p| p.get(property))
            .ok_or_else(|| error("missing configured geo id property"))?
    } else {
        field(v, "id")?
    };
    let id = id_string(id)?;
    let label = match o
        .label_property
        .as_ref()
        .and_then(|key| properties.and_then(|p| p.get(key)))
    {
        None | Some(Value::Null) => id.clone(),
        Some(Value::String(s)) => s.clone(),
        _ => return Err(error("geo label property must be a string")),
    };
    ensure(
        label.len() <= MAX_LABEL_BYTES,
        "geo label exceeds byte budget",
    )?;
    let value = match o
        .value_property
        .as_ref()
        .and_then(|key| properties.and_then(|p| p.get(key)))
    {
        None | Some(Value::Null) => o.missing_value,
        Some(value) => Some(number(value)?),
    };
    Ok((id, label, value))
}
fn coordinate(p: [f64; 2]) -> Result<[f64; 2]> {
    ensure(
        p[0].is_finite()
            && p[1].is_finite()
            && (-180. ..=180.).contains(&p[0])
            && (-90. ..=90.).contains(&p[1]),
        "geo coordinate must be finite longitude/latitude",
    )?;
    Ok(p)
}
fn position(v: &Value) -> Result<[f64; 2]> {
    let p = array(v)?;
    ensure(
        (2..=3).contains(&p.len()),
        "geo position must contain longitude, latitude and optional altitude",
    )?;
    if p.len() == 3 {
        number(&p[2])?;
    }
    coordinate([number(&p[0])?, number(&p[1])?])
}
fn valid_ring(ring: Ring) -> Result<Ring> {
    ensure(
        ring.len() >= 4 && ring.first() == ring.last(),
        "geo linear ring must have at least four positions and be closed",
    )?;
    let distinct = ring
        .iter()
        .map(|p| {
            (
                if p[0] == 0. { 0 } else { p[0].to_bits() },
                if p[1] == 0. { 0 } else { p[1].to_bits() },
            )
        })
        .collect::<BTreeSet<_>>();
    ensure(
        distinct.len() >= 3,
        "geo linear ring needs three distinct vertices",
    )?;
    Ok(ring)
}
fn geo_ring(v: &Value, b: &mut Budget) -> Result<Ring> {
    let values = array(v)?;
    ensure(
        values.len() <= b.max_points.saturating_sub(b.points),
        "geo point budget exceeded",
    )?;
    let mut points = Vec::with_capacity(values.len());
    for p in values {
        b.point()?;
        points.push(position(p)?);
    }
    valid_ring(points)
}
fn geo_polygon(v: &Value, b: &mut Budget) -> Result<Polygon> {
    let rings = array(v)?;
    ensure(
        !rings.is_empty() && rings.len() <= b.max_points / 4,
        "invalid polygon ring count",
    )?;
    rings.iter().map(|ring| geo_ring(ring, b)).collect()
}
fn geo_geometry(v: &Value, b: &mut Budget) -> Result<Vec<Polygon>> {
    match kind(v)? {
        "Polygon" => Ok(vec![geo_polygon(field(v, "coordinates")?, b)?]),
        "MultiPolygon" => {
            let polygons = array(field(v, "coordinates")?)?;
            ensure(
                !polygons.is_empty() && polygons.len() <= b.max_points / 4,
                "invalid multipolygon count",
            )?;
            polygons.iter().map(|p| geo_polygon(p, b)).collect()
        }
        _ => Err(error(
            "only Polygon and MultiPolygon geometries are supported",
        )),
    }
}
fn push_feature(
    out: &mut Vec<GeoFeature>,
    ids: &mut BTreeSet<String>,
    b: &mut Budget,
    v: &Value,
    polygons: Vec<Polygon>,
    o: &GeoParseOptions,
) -> Result<()> {
    b.feature()?;
    let (id, label, value) = metadata(v, o)?;
    ensure(ids.insert(id.clone()), "duplicate geo feature id")?;
    out.push(GeoFeature {
        id,
        label,
        value,
        polygons,
    });
    Ok(())
}
/// Parse a GeoJSON FeatureCollection (or a single Feature). Unsupported/null
/// geometries, missing identities, unclosed rings and malformed values reject
/// the complete proposal. Coordinates remain geographic; no projection or
/// antimeridian clipping is silently applied by ingestion.
pub fn parse_geojson(input: &str, o: &GeoParseOptions) -> Result<Vec<GeoFeature>> {
    let mut budget = Budget::new(o)?;
    let root = parse(input)?;
    let mut out = vec![];
    let mut ids = BTreeSet::new();
    let features: Vec<&Value> = match kind(&root)? {
        "FeatureCollection" => {
            let values = array(field(&root, "features")?)?;
            ensure(
                values.len() <= o.max_features,
                "geo feature budget exceeded",
            )?;
            values.iter().collect()
        }
        "Feature" => vec![&root],
        _ => return Err(error("expected GeoJSON Feature or FeatureCollection")),
    };
    for feature in features {
        ensure(
            kind(feature)? == "Feature",
            "FeatureCollection entries must be Features",
        )?;
        let polygons = geo_geometry(field(feature, "geometry")?, &mut budget)?;
        push_feature(&mut out, &mut ids, &mut budget, feature, polygons, o)?;
    }
    Ok(out)
}

#[derive(Clone, Copy)]
struct Transform {
    scale: [f64; 2],
    translate: [f64; 2],
}
fn pair(v: &Value) -> Result<[f64; 2]> {
    let a = array(v)?;
    ensure(a.len() == 2, "topology transform requires two coordinates")?;
    Ok([number(&a[0])?, number(&a[1])?])
}
fn topology_arcs(root: &Value, b: &mut Budget) -> Result<Vec<Ring>> {
    let transform = match root.get("transform") {
        None => None,
        Some(v) => Some(Transform {
            scale: pair(field(v, "scale")?)?,
            translate: pair(field(v, "translate")?)?,
        }),
    };
    let arcs = array(field(root, "arcs")?)?;
    ensure(
        arcs.len() <= b.max_points / 2,
        "topology arc budget exceeded",
    )?;
    let mut result = Vec::with_capacity(arcs.len());
    for arc in arcs {
        let positions = array(arc)?;
        ensure(
            positions.len() >= 2 && positions.len() <= b.max_points.saturating_sub(b.points),
            "topology arcs require two positions within point budget",
        )?;
        let mut decoded = Vec::with_capacity(positions.len());
        let mut accumulated = [0_i64; 2];
        for p in positions {
            b.point()?;
            let p = pair(p)?;
            let p = if let Some(t) = transform {
                for i in 0..2 {
                    ensure(
                        p[i].fract() == 0. && p[i].abs() <= 9_007_199_254_740_991.,
                        "quantized topology deltas must be exact integers",
                    )?;
                    accumulated[i] = accumulated[i]
                        .checked_add(p[i] as i64)
                        .ok_or_else(|| error("topology delta overflow"))?;
                    ensure(
                        accumulated[i].unsigned_abs() <= 9_007_199_254_740_991,
                        "topology cumulative coordinate exceeds exact integer range",
                    )?;
                }
                [
                    accumulated[0] as f64 * t.scale[0] + t.translate[0],
                    accumulated[1] as f64 * t.scale[1] + t.translate[1],
                ]
            } else {
                p
            };
            decoded.push(coordinate(p)?);
        }
        result.push(decoded);
    }
    Ok(result)
}
fn topo_ring(v: &Value, arcs: &[Ring], b: &mut Budget) -> Result<Ring> {
    let references = array(v)?;
    ensure(
        !references.is_empty() && references.len() <= b.max_points,
        "topology ring arc references exceed budget",
    )?;
    let mut points = Vec::new();
    for reference in references {
        let reference = reference
            .as_i64()
            .ok_or_else(|| error("topology arc reference must be a signed integer"))?;
        let index = if reference < 0 { !reference } else { reference };
        let index = usize::try_from(index).map_err(|_| error("topology arc index overflow"))?;
        let arc = arcs
            .get(index)
            .ok_or_else(|| error("topology arc index out of range"))?;
        let reversed = reference < 0;
        let first = if reversed { arc.last() } else { arc.first() };
        if let Some(last) = points.last() {
            ensure(
                Some(last) == first,
                "topology arcs have disconnected endpoints",
            )?;
        }
        let skip = usize::from(!points.is_empty());
        ensure(
            arc.len().saturating_sub(skip) <= b.max_points.saturating_sub(b.points),
            "expanded topology point budget exceeded",
        )?;
        if reversed {
            for p in arc.iter().rev().skip(skip) {
                b.point()?;
                points.push(*p);
            }
        } else {
            for p in arc.iter().skip(skip) {
                b.point()?;
                points.push(*p);
            }
        }
    }
    valid_ring(points)
}
fn topo_polygon(v: &Value, arcs: &[Ring], b: &mut Budget) -> Result<Polygon> {
    let rings = array(v)?;
    ensure(
        !rings.is_empty() && rings.len() <= b.max_points / 4,
        "invalid topology polygon ring count",
    )?;
    rings.iter().map(|r| topo_ring(r, arcs, b)).collect()
}
fn topo_object(
    v: &Value,
    arcs: &[Ring],
    b: &mut Budget,
    out: &mut Vec<GeoFeature>,
    ids: &mut BTreeSet<String>,
    o: &GeoParseOptions,
    depth: usize,
) -> Result<()> {
    ensure(depth <= 16, "topology collection nesting exceeds 16 levels")?;
    let polygons = match kind(v)? {
        "GeometryCollection" => {
            let geometries = array(field(v, "geometries")?)?;
            ensure(
                geometries.len() <= o.max_features,
                "topology geometry budget exceeded",
            )?;
            for geometry in geometries {
                topo_object(geometry, arcs, b, out, ids, o, depth + 1)?;
            }
            return Ok(());
        }
        "Polygon" => vec![topo_polygon(field(v, "arcs")?, arcs, b)?],
        "MultiPolygon" => {
            let polygons = array(field(v, "arcs")?)?;
            ensure(
                !polygons.is_empty() && polygons.len() <= b.max_points / 4,
                "invalid topology multipolygon count",
            )?;
            polygons
                .iter()
                .map(|p| topo_polygon(p, arcs, b))
                .collect::<Result<_>>()?
        }
        _ => {
            return Err(error(
                "only polygon topology objects and collections are supported",
            ));
        }
    };
    push_feature(out, ids, b, v, polygons, o)
}
/// Select one named TopoJSON object. GeometryCollection entries become features;
/// Polygon and MultiPolygon objects retain their own metadata. Both source arc
/// decoding and expanded output enforce independent point budgets.
pub fn parse_topojson(
    input: &str,
    object_name: &str,
    o: &GeoParseOptions,
) -> Result<Vec<GeoFeature>> {
    let mut source_budget = Budget::new(o)?;
    let root = parse(input)?;
    ensure(kind(&root)? == "Topology", "expected TopoJSON Topology")?;
    let objects = object(field(&root, "objects")?)?;
    let selected = objects
        .get(object_name)
        .ok_or_else(|| error("topology object name not found"))?;
    let arcs = topology_arcs(&root, &mut source_budget)?;
    let mut output_budget = Budget::new(o)?;
    let mut out = vec![];
    let mut ids = BTreeSet::new();
    topo_object(
        selected,
        &arcs,
        &mut output_budget,
        &mut out,
        &mut ids,
        o,
        0,
    )?;
    Ok(out)
}
