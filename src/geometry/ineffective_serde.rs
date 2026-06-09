use serde::{Deserialize, Deserializer, Serialize, Serializer};

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
}
