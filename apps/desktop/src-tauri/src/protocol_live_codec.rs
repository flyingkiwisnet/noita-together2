// Bounded schema-driven codec, derived from the existing owned strict codec.
#[derive(Debug, PartialEq, Eq)]
pub enum ProtocolError {
    TooLarge,
    InvalidMessage,
    VersionMismatch,
}

fn validate(
    name: &str,
    value: &Value,
    depth: usize,
    fields: &mut usize,
) -> Result<(), ProtocolError> {
    let limits = &SCHEMA["limits"];
    if depth > limits["depth"].as_u64().unwrap() as usize {
        return Err(ProtocolError::InvalidMessage);
    }
    let definition = &SCHEMA["messages"][name];
    if !definition.is_object() {
        return Err(ProtocolError::InvalidMessage);
    }
    if name == "Config"
        && serde_json::to_vec(value)
            .map_err(|_| ProtocolError::InvalidMessage)?
            .len()
            > 8192
    {
        return Err(ProtocolError::InvalidMessage);
    }
    let object = value.as_object().ok_or(ProtocolError::InvalidMessage)?;
    *fields = fields
        .checked_add(object.len())
        .ok_or(ProtocolError::InvalidMessage)?;
    if *fields > limits["fields"].as_u64().unwrap() as usize {
        return Err(ProtocolError::InvalidMessage);
    }
    for (key, field) in definition["fields"].as_object().ok_or(ProtocolError::InvalidMessage)? { if field["required"]==true && !object.contains_key(key){return Err(ProtocolError::InvalidMessage);} }
    for (key, item) in object {
        let field = &definition["fields"][key];
        let kind = field["type"]
            .as_str()
            .ok_or(ProtocolError::InvalidMessage)?;
        let check = |item: &Value, fields: &mut usize, maximum: Option<u64>| match kind {
            "string" => {
                let text = item.as_str().ok_or(ProtocolError::InvalidMessage)?;
                if text.len() as u64 > maximum.ok_or(ProtocolError::InvalidMessage)? {
                    return Err(ProtocolError::InvalidMessage);
                }
                Ok(())
            }
            "uint32" => match item.as_u64() {
                Some(n) if n <= u32::MAX as u64 => Ok(()),
                _ => Err(ProtocolError::InvalidMessage),
            },
            "bool" if item.is_boolean() => Ok(()),
            "bool" => Err(ProtocolError::InvalidMessage),
            _ => validate(kind, item, depth + 1, fields),
        };
        if field["repeated"].as_bool() == Some(true) {
            let items = item.as_array().ok_or(ProtocolError::InvalidMessage)?;
            if items.len() as u64 > field["max"].as_u64().ok_or(ProtocolError::InvalidMessage)? {
                return Err(ProtocolError::InvalidMessage);
            }
            for item in items {
                check(item, fields, field["itemMax"].as_u64())?;
            }
        } else {
            check(item, fields, field["max"].as_u64())?;
        }
    }
    if let Some(commands) = definition["oneof"].as_array() {
        if commands
            .iter()
            .filter(|key| object.contains_key(key.as_str().unwrap()))
            .count()
            != 1
        {
            return Err(ProtocolError::InvalidMessage);
        }
    }
    Ok(())
}
struct StrictValue(Value);
pub fn json_value(name:&str,bytes:&[u8])->Result<Value,ProtocolError>{
 if bytes.len()>32768{return Err(ProtocolError::TooLarge);}let mut decoder=serde_json::Deserializer::from_slice(bytes);
 let value=<StrictValue as serde::Deserialize>::deserialize(&mut decoder).map_err(|_|ProtocolError::InvalidMessage)?.0;decoder.end().map_err(|_|ProtocolError::InvalidMessage)?;validate(name,&value,0,&mut 0)?;Ok(value)
}
impl<'de> serde::Deserialize<'de> for StrictValue {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = StrictValue;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("unique JSON object keys")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut a: A,
            ) -> Result<Self::Value, A::Error> {
                let mut m = serde_json::Map::new();
                while let Some(k) = a.next_key::<String>()? {
                    if m.contains_key(&k) {
                        return Err(serde::de::Error::custom("duplicate key"));
                    }
                    m.insert(k, a.next_value::<StrictValue>()?.0);
                }
                Ok(StrictValue(Value::Object(m)))
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut a: A,
            ) -> Result<Self::Value, A::Error> {
                let mut v = Vec::new();
                while let Some(n) = a.next_element::<StrictValue>()? {
                    v.push(n.0)
                }
                Ok(StrictValue(Value::Array(v)))
            }
            fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(StrictValue(v.into()))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(StrictValue(v.into()))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(StrictValue(v.into()))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Self::Value, E> {
                Ok(StrictValue(
                    serde_json::Number::from_f64(v)
                        .ok_or_else(|| E::custom("number"))?
                        .into(),
                ))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(StrictValue(v.into()))
            }
            fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Self::Value, E> {
                Ok(StrictValue(v.into()))
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::Null))
            }
        }
        d.deserialize_any(Visitor)
    }
}

pub fn binary_value(name: &str, bytes: &[u8]) -> Result<Value, ProtocolError> {
    if bytes.len() > SCHEMA["limits"]["bytes"].as_u64().unwrap() as usize {
        return Err(ProtocolError::TooLarge);
    }
    fn varint(bytes: &[u8], pos: &mut usize) -> Result<u32, ProtocolError> {
        let mut n = 0u64;
        for i in 0..5 {
            let b = *bytes.get(*pos).ok_or(ProtocolError::InvalidMessage)?;
            *pos += 1;
            n |= ((b & 127) as u64) << (7 * i);
            if b & 128 == 0 {
                if i > 0 && b == 0 || n > u32::MAX as u64 {
                    return Err(ProtocolError::InvalidMessage);
                }
                return Ok(n as u32);
            }
        }
        Err(ProtocolError::InvalidMessage)
    }
    fn scan(
        name: &str,
        bytes: &[u8],
        depth: usize,
        budget: &mut usize,
    ) -> Result<Value, ProtocolError> {
        if depth > SCHEMA["limits"]["depth"].as_u64().unwrap() as usize {
            return Err(ProtocolError::InvalidMessage);
        }
        let def = &SCHEMA["messages"][name];
        let fields = def["fields"]
            .as_object()
            .ok_or(ProtocolError::InvalidMessage)?;
        let mut out = serde_json::Map::new();
        let mut p = 0;
        while p < bytes.len() {
            *budget += 1;
            if *budget > SCHEMA["limits"]["fields"].as_u64().unwrap() as usize {
                return Err(ProtocolError::InvalidMessage);
            }
            let tag = varint(bytes, &mut p)?;
            let (key, f) = fields
                .iter()
                .find(|(_, f)| f["id"].as_u64() == Some((tag >> 3) as u64))
                .ok_or(ProtocolError::InvalidMessage)?;
            let repeated = f["repeated"] == true;
            if !repeated && out.contains_key(key) {
                return Err(ProtocolError::InvalidMessage);
            }
            let kind = f["type"].as_str().ok_or(ProtocolError::InvalidMessage)?;
            let v = if kind == "bool" || kind == "uint32" {
                if tag & 7 != 0 {
                    return Err(ProtocolError::InvalidMessage);
                }
                let n = varint(bytes, &mut p)?;
                if kind == "bool" {
                    if n > 1 {
                        return Err(ProtocolError::InvalidMessage);
                    }
                    Value::Bool(n == 1)
                } else {
                    n.into()
                }
            } else {
                if tag & 7 != 2 {
                    return Err(ProtocolError::InvalidMessage);
                }
                let len = varint(bytes, &mut p)? as usize;
                let end = p
                    .checked_add(len)
                    .filter(|end| *end <= bytes.len())
                    .ok_or(ProtocolError::InvalidMessage)?;
                let v = if kind == "string" {
                    std::str::from_utf8(&bytes[p..end])
                        .map_err(|_| ProtocolError::InvalidMessage)?
                        .into()
                } else {
                    scan(kind, &bytes[p..end], depth + 1, budget)?
                };
                p = end;
                v
            };
            if repeated {
                out.entry(key.clone())
                    .or_insert_with(|| Value::Array(Vec::new()))
                    .as_array_mut()
                    .unwrap()
                    .push(v);
            } else {
                out.insert(key.clone(), v);
            }
        }
        let value = Value::Object(out);
        validate(name, &value, depth, &mut 0)?;
        Ok(value)
    }
    scan(name, bytes, 0, &mut 0)
}


