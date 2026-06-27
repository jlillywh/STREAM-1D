use crate::geometry::{IneffectiveBlock, IneffectiveFlowAreas};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

fn blocks_from_station_elevation_pairs(
    pairs: &[serde_json::Value],
) -> Result<Vec<IneffectiveBlock>, String> {
    pairs
        .iter()
        .map(|pair| {
            let arr = pair
                .as_array()
                .ok_or_else(|| "ineffective block must be [station, elevation]".to_string())?;
            if arr.len() != 2 {
                return Err("ineffective block must be [station, elevation]".to_string());
            }
            let station = arr[0]
                .as_f64()
                .ok_or_else(|| "ineffective station must be a number".to_string())?;
            let elevation = arr[1]
                .as_f64()
                .ok_or_else(|| "ineffective elevation must be a number".to_string())?;
            Ok(IneffectiveBlock { station, elevation })
        })
        .collect()
}

fn parse_side_pair_arrays(
    obj: &serde_json::Map<String, serde_json::Value>,
    side: &str,
) -> Result<Vec<IneffectiveBlock>, String> {
    let stations_key = format!("{side}_stations");
    let elevations_key = format!("{side}_elevations");
    let stations = obj
        .get(&stations_key)
        .and_then(|v| v.as_array())
        .ok_or_else(|| format!("missing or invalid {stations_key}"))?;
    let elevations = obj
        .get(&elevations_key)
        .and_then(|v| v.as_array())
        .ok_or_else(|| format!("missing or invalid {elevations_key}"))?;
    if stations.len() != elevations.len() {
        return Err(format!(
            "{stations_key} and {elevations_key} must have the same length"
        ));
    }
    stations
        .iter()
        .zip(elevations.iter())
        .map(|(s, e)| {
            Ok(IneffectiveBlock {
                station: s
                    .as_f64()
                    .ok_or_else(|| format!("{stations_key} values must be numbers"))?,
                elevation: e
                    .as_f64()
                    .ok_or_else(|| format!("{elevations_key} values must be numbers"))?,
            })
        })
        .collect()
}

/// Deserialize `CrossSection.ineffective_flow_areas` / `ineffective_areas`.
///
/// Accepts:
/// - Canonical: `{ left_blocks, right_blocks }` with `{ station, elevation }` objects
/// - Parallel pairs: `{ left_stations, left_elevations, right_stations, right_elevations }`
/// - Nested pairs: `{ left: [[station, elevation], ...], right: [...] }`
pub fn deserialize_ineffective_flow_areas_option<'de, D>(
    deserializer: D,
) -> Result<Option<IneffectiveFlowAreas>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Option<serde_json::Value> = Option::deserialize(deserializer)?;
    match value {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::Object(obj)) => {
            if obj.contains_key("left_blocks") || obj.contains_key("right_blocks") {
                return IneffectiveFlowAreas::deserialize(serde_json::Value::Object(obj))
                    .map(Some)
                    .map_err(serde::de::Error::custom);
            }
            if obj.contains_key("left_stations") || obj.contains_key("right_stations") {
                let left_blocks = if obj.contains_key("left_stations") {
                    parse_side_pair_arrays(&obj, "left").map_err(serde::de::Error::custom)?
                } else {
                    vec![]
                };
                let right_blocks = if obj.contains_key("right_stations") {
                    parse_side_pair_arrays(&obj, "right").map_err(serde::de::Error::custom)?
                } else {
                    vec![]
                };
                if left_blocks.is_empty() && right_blocks.is_empty() {
                    return Ok(None);
                }
                return Ok(Some(IneffectiveFlowAreas {
                    left_blocks,
                    right_blocks,
                }));
            }
            if obj.contains_key("left") || obj.contains_key("right") {
                let left_blocks = match obj.get("left") {
                    Some(serde_json::Value::Array(arr)) => blocks_from_station_elevation_pairs(arr)
                        .map_err(serde::de::Error::custom)?,
                    _ => vec![],
                };
                let right_blocks = match obj.get("right") {
                    Some(serde_json::Value::Array(arr)) => blocks_from_station_elevation_pairs(arr)
                        .map_err(serde::de::Error::custom)?,
                    _ => vec![],
                };
                if left_blocks.is_empty() && right_blocks.is_empty() {
                    return Ok(None);
                }
                return Ok(Some(IneffectiveFlowAreas {
                    left_blocks,
                    right_blocks,
                }));
            }
            Err(serde::de::Error::custom(
                "ineffective areas must use left_blocks/right_blocks, left_stations/left_elevations pairs, or left/right [[station, elevation], ...]",
            ))
        }
        _ => Err(serde::de::Error::custom(
            "ineffective areas must be an object",
        )),
    }
}

/// Serialize canonical `{ left_blocks, right_blocks }` form.
pub fn serialize_ineffective_flow_areas_option<S>(
    value: &Option<IneffectiveFlowAreas>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        None => serializer.serialize_none(),
        Some(areas) => areas.serialize(serializer),
    }
}

/// Deserialize per-bridge ineffective block arrays.
///
/// Accepts legacy flat `[s0, s1, …]` (one block per bridge) or nested `[[s0, s1], [s2]]`.
pub fn deserialize_bridge_block_arrays<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<Vec<f64>>>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Option<serde_json::Value> = Option::deserialize(deserializer)?;
    match value {
        None => Ok(None),
        Some(serde_json::Value::Array(arr)) if arr.is_empty() => Ok(Some(vec![])),
        Some(serde_json::Value::Array(arr)) => {
            if arr.iter().all(|v| v.is_number()) {
                let mut blocks = Vec::with_capacity(arr.len());
                for v in &arr {
                    let n = v.as_f64().ok_or_else(|| {
                        serde::de::Error::custom("ineffective station must be a number")
                    })?;
                    blocks.push(vec![n]);
                }
                Ok(Some(blocks))
            } else if arr.iter().all(|v| v.is_array()) {
                Ok(Some(
                    arr.iter()
                        .map(|bridge| {
                            bridge
                                .as_array()
                                .ok_or_else(|| {
                                    serde::de::Error::custom("expected array of block arrays")
                                })?
                                .iter()
                                .map(|v| {
                                    v.as_f64().ok_or_else(|| {
                                        serde::de::Error::custom(
                                            "ineffective block value must be a number",
                                        )
                                    })
                                })
                                .collect::<Result<Vec<f64>, _>>()
                        })
                        .collect::<Result<Vec<Vec<f64>>, _>>()?,
                ))
            } else {
                Err(serde::de::Error::custom(
                    "ineffective blocks must be a flat number array (one block per bridge) or nested array of block arrays",
                ))
            }
        }
        _ => Err(serde::de::Error::custom(
            "ineffective blocks must be an array",
        )),
    }
}

/// Serialize per-bridge ineffective blocks; flatten when every bridge has exactly one block.
pub fn serialize_bridge_block_arrays<S>(
    value: &Option<Vec<Vec<f64>>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        None => serializer.serialize_none(),
        Some(blocks) if blocks.iter().all(|b| b.len() == 1) => {
            let flat: Vec<f64> = blocks.iter().map(|b| b[0]).collect();
            flat.serialize(serializer)
        }
        Some(blocks) => blocks.serialize(serializer),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    struct Wrapper {
        #[serde(
            default,
            deserialize_with = "deserialize_bridge_block_arrays",
            serialize_with = "serialize_bridge_block_arrays"
        )]
        blocks: Option<Vec<Vec<f64>>>,
    }

    #[test]
    fn flat_json_deserializes_to_single_block_per_bridge() {
        let w: Wrapper = serde_json::from_str(r#"{"blocks":[30.0,40.0]}"#).unwrap();
        assert_eq!(w.blocks.unwrap(), vec![vec![30.0], vec![40.0]]);
    }

    #[test]
    fn nested_json_roundtrip() {
        let w: Wrapper = serde_json::from_str(r#"{"blocks":[[20.0,30.0],[40.0]]}"#).unwrap();
        let out = serde_json::to_string(&w).unwrap();
        assert!(out.contains("[[20.0,30.0],[40.0]]") || out.contains("[[20.0, 30.0], [40.0]]"));
        let again: Wrapper = serde_json::from_str(&out).unwrap();
        assert_eq!(again.blocks, w.blocks);
    }

    #[test]
    fn single_block_serializes_flat() {
        let w = Wrapper {
            blocks: Some(vec![vec![30.0], vec![40.0]]),
        };
        let out = serde_json::to_string(&w).unwrap();
        assert!(out.contains("[30.0,40.0]") || out.contains("[30.0, 40.0]"));
        assert!(!out.contains("[[30.0]]"));
    }

    #[derive(Deserialize)]
    struct XsIneffective {
        #[serde(
            default,
            alias = "ineffective_areas",
            deserialize_with = "deserialize_ineffective_flow_areas_option"
        )]
        ineffective_flow_areas: Option<IneffectiveFlowAreas>,
    }

    #[test]
    fn cross_section_accepts_ineffective_areas_alias_with_blocks() {
        let xs: XsIneffective = serde_json::from_str(
            r#"{"ineffective_areas":{"left_blocks":[{"station":5.0,"elevation":4.0}],"right_blocks":[]}}"#,
        )
        .unwrap();
        let areas = xs.ineffective_flow_areas.unwrap();
        assert_eq!(areas.left_blocks.len(), 1);
        assert!((areas.left_blocks[0].station - 5.0).abs() < 1e-9);
    }

    #[test]
    fn cross_section_accepts_parallel_station_elevation_pairs() {
        let xs: XsIneffective = serde_json::from_str(
            r#"{"ineffective_flow_areas":{"left_stations":[5.0,10.0],"left_elevations":[4.0,4.5],"right_stations":[35.0],"right_elevations":[5.0]}}"#,
        )
        .unwrap();
        let areas = xs.ineffective_flow_areas.unwrap();
        assert_eq!(areas.left_blocks.len(), 2);
        assert_eq!(areas.right_blocks.len(), 1);
    }

    #[test]
    fn cross_section_accepts_nested_station_elevation_pairs() {
        let xs: XsIneffective = serde_json::from_str(
            r#"{"ineffective_areas":{"left":[[5.0,4.0],[10.0,4.5]],"right":[[35.0,5.0]]}}"#,
        )
        .unwrap();
        let areas = xs.ineffective_flow_areas.unwrap();
        assert_eq!(areas.left_blocks.len(), 2);
        assert!((areas.right_blocks[0].elevation - 5.0).abs() < 1e-9);
    }

    #[derive(Serialize)]
    struct XsOut {
        #[serde(
            default,
            serialize_with = "serialize_ineffective_flow_areas_option",
            skip_serializing_if = "Option::is_none"
        )]
        ineffective_flow_areas: Option<IneffectiveFlowAreas>,
    }

    #[test]
    fn cross_section_ineffective_serializes_canonical_blocks() {
        let areas =
            IneffectiveFlowAreas::from_block_pairs(&[5.0], &[4.0], &[35.0], &[5.0]).unwrap();
        let out = serde_json::to_string(&XsOut {
            ineffective_flow_areas: Some(areas),
        })
        .unwrap();
        assert!(out.contains("left_blocks"));
        assert!(out.contains("right_blocks"));
    }

    #[test]
    fn cross_section_rejects_mismatched_station_elevation_lengths() {
        assert!(serde_json::from_str::<XsIneffective>(
            r#"{"ineffective_flow_areas":{"left_stations":[5.0],"left_elevations":[4.0,4.5]}}"#,
        )
        .is_err());
    }

    #[test]
    fn cross_section_null_ineffective_deserializes_none() {
        let xs: XsIneffective = serde_json::from_str(r#"{"ineffective_flow_areas":null}"#).unwrap();
        assert!(xs.ineffective_flow_areas.is_none());
    }

    #[test]
    fn cross_section_rejects_non_object_ineffective() {
        assert!(serde_json::from_str::<XsIneffective>(r#"{"ineffective_flow_areas":[]}"#).is_err());
    }

    #[test]
    fn cross_section_empty_side_arrays_deserialize_none() {
        let xs: XsIneffective = serde_json::from_str(
            r#"{"ineffective_flow_areas":{"left_stations":[],"left_elevations":[]}}"#,
        )
        .unwrap();
        assert!(xs.ineffective_flow_areas.is_none());
    }

    #[test]
    fn cross_section_serializes_none_as_absent() {
        let out = serde_json::to_string(&XsOut {
            ineffective_flow_areas: None,
        })
        .unwrap();
        assert!(!out.contains("ineffective_flow_areas"));
    }

    #[test]
    fn bridge_block_arrays_reject_mixed_flat_and_nested() {
        assert!(serde_json::from_str::<Wrapper>(r#"{"blocks":[30.0,[40.0]]}"#).is_err());
    }
}
