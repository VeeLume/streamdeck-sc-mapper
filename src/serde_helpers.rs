use serde::{Deserialize, Deserializer};
use serde_json::Value;

pub fn opt_u64_from_str_or_num<'de, D>(de: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    let v = Option::<Value>::deserialize(de)?;
    Ok(match v {
        None | Some(Value::Null) => None,
        Some(Value::Number(n)) => n.as_u64(),
        Some(Value::String(s)) => {
            let s = s.trim();
            if s.is_empty() { None } else { s.parse().ok() }
        }
        _ => None,
    })
}

pub fn u64_from_str_or_num_default_200<'de, D>(de: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let v = Option::<Value>::deserialize(de)?;
    Ok(match v {
        None | Some(Value::Null) => 200,
        Some(Value::Number(n)) => n.as_u64().unwrap_or(200),
        Some(Value::String(s)) => s.trim().parse::<u64>().unwrap_or(200),
        _ => 200,
    })
}
