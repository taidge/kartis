//! Test-only bulk numeric probe: no chart, DOM or global application state.
//! Special IEEE values are explicit strings; JSON decimal parsing preserves the
//! original frozen f64 inputs with serde_json's float_roundtrip feature.
use kartis::{
    ContinuousScaleOptions, PiecewiseScaleOptions, ResolvedContinuousScale, ScaleKind, ScaleNice,
};
use serde_json::{Value, json};
use wasm_bindgen::prelude::*;

fn error(message: &str) -> JsValue {
    JsValue::from_str(message)
}
fn number(v: &Value) -> std::result::Result<f64, JsValue> {
    v.as_f64()
        .or_else(|| v.as_str()?.parse().ok())
        .ok_or_else(|| error("invalid numeric probe input"))
}
fn array(v: &Value) -> std::result::Result<&[Value], JsValue> {
    v.as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| error("invalid probe array"))
}
fn numbers(v: &Value) -> std::result::Result<Vec<f64>, JsValue> {
    array(v)?.iter().map(number).collect()
}
fn encode(v: f64) -> Value {
    if v.is_nan() {
        json!("NaN")
    } else if v == f64::INFINITY {
        json!("Infinity")
    } else if v == f64::NEG_INFINITY {
        json!("-Infinity")
    } else if v == 0. && v.is_sign_negative() {
        json!("-0")
    } else {
        json!(v)
    }
}
fn encoded(values: &[f64]) -> Vec<Value> {
    values.iter().copied().map(encode).collect()
}
fn inverse(value: kartis::Result<f64>) -> Value {
    match value {
        Ok(v) => json!({"ok":true,"value":encode(v)}),
        Err(e) => json!({"ok":false,"error":e.to_string()}),
    }
}
fn boolean(v: &Value) -> std::result::Result<bool, JsValue> {
    v.as_bool().ok_or_else(|| error("invalid probe option"))
}
fn options(c: &Value) -> std::result::Result<PiecewiseScaleOptions, JsValue> {
    let parameter = number(&c["parameter"])?;
    let kind = match c["kind"].as_str() {
        Some("linear") => ScaleKind::Linear,
        Some("log") => ScaleKind::Log,
        Some("symlog") => ScaleKind::SymLog {
            constant: parameter,
        },
        Some("pow") => ScaleKind::Pow {
            exponent: parameter,
        },
        Some("sqrt") => ScaleKind::Sqrt,
        Some("utc") => ScaleKind::Utc,
        _ => return Err(error("unknown probe scale")),
    };
    let o = &c["options"];
    let nice = match o["nice"].as_i64() {
        Some(-1) => ScaleNice::Off,
        Some(0) => ScaleNice::Default,
        Some(v) if v > 0 => ScaleNice::Count(v as usize),
        _ => return Err(error("invalid probe nice option")),
    };
    Ok(PiecewiseScaleOptions {
        domain: numbers(&c["domain"])?,
        range: numbers(&c["range"])?,
        continuous: ContinuousScaleOptions {
            kind,
            nice,
            reverse: boolean(&o["reverse"])?,
            round: boolean(&o["round"])?,
            clamp: boolean(&o["clamp"])?,
            log_base: if kind == ScaleKind::Log {
                parameter
            } else {
                10.
            },
        },
    })
}

#[wasm_bindgen]
pub fn piecewise_repeated_probe(input: &str) -> std::result::Result<String, JsValue> {
    if input.len() > 2_000_000 {
        return Err(error("numeric probe byte budget exceeded"));
    }
    let input: Value =
        serde_json::from_str(input).map_err(|e| error(&format!("numeric probe JSON: {e}")))?;
    let cases = array(&input)?;
    if cases.len() > 256 {
        return Err(error("numeric probe batch budget exceeded"));
    }
    let mut output = Vec::with_capacity(cases.len());
    for case in cases {
        let index = case["index"]
            .as_u64()
            .ok_or_else(|| error("missing probe index"))?;
        let profile = options(case)?;
        let scale = match ResolvedContinuousScale::piecewise(&profile) {
            Ok(scale) => scale,
            Err(e) => {
                output.push(json!({"index":index,"accepted":false,"error":e.to_string()}));
                continue;
            }
        };
        let mut raw_profile = profile.clone();
        raw_profile.continuous.round = false;
        let raw =
            ResolvedContinuousScale::piecewise(&raw_profile).map_err(|e| error(&e.to_string()))?;
        let mapped = numbers(&case["queries"])?
            .into_iter()
            .map(|v| json!({"value":encode(scale.map(v)),"unrounded":encode(raw.map(v))}))
            .collect::<Vec<_>>();
        let inverses = numbers(&case["inversePixels"])?
            .into_iter()
            .map(|v| inverse(scale.invert(v)))
            .collect::<Vec<_>>();
        let branches = array(&case["branches"])?
            .iter()
            .map(|branch| {
                let index = branch["index"]
                    .as_u64()
                    .ok_or_else(|| error("missing probe branch index"))?
                    as usize;
                let values = numbers(&branch["pixels"])?
                    .into_iter()
                    .map(|v| inverse(scale.invert_segment(index, v)))
                    .collect::<Vec<_>>();
                Ok(json!({"index":index,"values":values}))
            })
            .collect::<std::result::Result<Vec<_>, JsValue>>()?;
        output.push(json!({"index":index,"accepted":true,"source":encoded(scale.source_domain_stops()),"display":encoded(scale.display_domain_stops()),"range":encoded(scale.range_stops()),"invertible":scale.is_invertible(),"mapped":mapped,"inverse":inverses,"branches":branches,"ticks":encoded(&scale.ticks(7))}));
    }
    serde_json::to_string(&output).map_err(|e| error(&e.to_string()))
}
