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
    if definition["fields"].as_object().is_some_and(|defs|defs.iter().any(|(key,field)|field["required"]==true&&!object.contains_key(key))){return Err(ProtocolError::InvalidMessage)}
    // Match the canonical G2 Config semantics for both JSON and binary input.
    // Older contracts without seedPolicy retain their own Config definition.
    if name=="Config"&&definition["fields"]["seedPolicy"].is_object(){
        match object.get("seedPolicy").and_then(Value::as_str){
            Some("individual-random") if value.get("seed").is_some_and(|seed|seed.as_u64()!=Some(0))||value["randomizeLoot"]==true=>return Err(ProtocolError::InvalidMessage),
            Some("shared-fixed") if value["seed"].as_u64().is_none_or(|seed|seed==0)=>return Err(ProtocolError::InvalidMessage),
            Some("shared-fixed"|"shared-random"|"individual-random")=>{},
            None if !object.contains_key("seedPolicy")&&!object.contains_key("randomizeLoot")=>{},
            _=>return Err(ProtocolError::InvalidMessage),
        }
    }
    *fields = fields
        .checked_add(object.len())
        .ok_or(ProtocolError::InvalidMessage)?;
    if *fields > limits["fields"].as_u64().unwrap() as usize {
        return Err(ProtocolError::InvalidMessage);
    }
    for (key, item) in object {
        let field = &definition["fields"][key];
        let kind = field["type"]
            .as_str()
            .ok_or(ProtocolError::InvalidMessage)?;
        let check = |item: &Value, fields: &mut usize, maximum: Option<u64>| match kind {
            "string" => {
                let text = item.as_str().ok_or(ProtocolError::InvalidMessage)?;
                if text.len() as u64 > maximum.ok_or(ProtocolError::InvalidMessage)?
                    ||field["values"].as_array().is_some_and(|values|!values.iter().any(|v|v.as_str()==Some(text))) {
                    return Err(ProtocolError::InvalidMessage);
                }
                Ok(())
            }
            "uint32" => match item.as_u64() {
                Some(n) if n <= field["maxValue"].as_u64().unwrap_or(u32::MAX as u64) => Ok(()),
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
pub fn json_value(name: &str, bytes: &[u8]) -> Result<Value, ProtocolError> {
    if bytes.len() as u64 > SCHEMA["limits"]["bytes"].as_u64().unwrap() {
        return Err(ProtocolError::TooLarge);
    }
    // serde_json::Value alone would silently retain only the last duplicate key.
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    let value = <StrictValue as serde::Deserialize>::deserialize(&mut decoder)
        .map_err(|_| ProtocolError::InvalidMessage)?
        .0;
    decoder.end().map_err(|_| ProtocolError::InvalidMessage)?;
    validate(name, &value, 0, &mut 0)?;
    Ok(value)
}
pub fn typed<T: DeserializeOwned>(name: &str, bytes: &[u8]) -> Result<T, ProtocolError> {
    serde_json::from_value(json_value(name, bytes)?).map_err(|_| ProtocolError::InvalidMessage)
}
fn decode<T: DeserializeOwned>(name: &str, bytes: &[u8]) -> Result<T, ProtocolError> {
    let value = json_value(name, bytes)?;
    if value["major"].as_u64() != SCHEMA["major"].as_u64()
        || value["revision"] != SCHEMA["revision"]
    {
        return Err(ProtocolError::VersionMismatch);
    }
    for field in ["serviceId", "requestId"] {
        if value[field].as_str().is_none_or(str::is_empty) {
            return Err(ProtocolError::InvalidMessage);
        }
    }
    serde_json::from_value(value).map_err(|_| ProtocolError::InvalidMessage)
}
pub fn request(bytes: &[u8]) -> Result<wire::Request, ProtocolError> {
    decode("Request", bytes)
}
pub fn response_as<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, ProtocolError> {
    decode("Response", bytes)
}
pub fn response(bytes: &[u8]) -> Result<wire::Response, ProtocolError> {
    decode("Response", bytes)
}

struct StrictValue(Value);
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
    if bytes.len() > 65536 {
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
        if depth > 8 {
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
            if *budget > 2048 {
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

#[cfg(test)]
mod golden_tests {
    use super::*;
    fn hex(s: &str) -> Vec<u8> {
        s.as_bytes()
            .chunks_exact(2)
            .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
            .collect()
    }
    #[test]
    fn server_owned_json_and_binary_goldens() {
        let gold: Value = serde_json::from_str(GOLDENS).unwrap();
        for f in gold["fixtures"].as_array().unwrap() {
            let name = f["type"].as_str().unwrap();
            let json = json_value(name, &serde_json::to_vec(&f["json"]).unwrap()).unwrap();
            assert_eq!(
                binary_value(name, &hex(f["hex"].as_str().unwrap())).unwrap(),
                json
            );
        }
        for f in gold["rejected"].as_array().unwrap() {
            let name = f["type"].as_str().unwrap();
            if let Some(h) = f["hex"].as_str() {
                assert!(binary_value(name, &hex(h)).is_err());
            } else {
                assert!(json_value(name, &serde_json::to_vec(&f["json"]).unwrap()).is_err());
            }
        }
    }
    #[test]
    fn duplicate_json_and_overlong_varints_rejected() {
        assert!(json_value("PairBegin", br#"{"deviceId":"a","deviceId":"b"}"#).is_err());
        assert!(binary_value("Request", &[0x88, 0, 1]).is_err());
        assert!(binary_value("Presence", &[8, 2]).is_err());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn base() -> Value {
        if SCHEMA["messages"]["Request"]["fields"]["arenaBind"].is_object(){return serde_json::json!({"major":1,"revision":SCHEMA["revision"],"serviceId":"fixture","requestId":"request-1","publish":{"eventId":"11111111-1111-4111-8111-111111111111","rulesRevision":1,"progression":0,"payload":{"steve":{"progression":0}},"senderDisposition":"applied"}});}
        if SCHEMA["messages"]["Request"]["fields"]["commit"].is_object() || SCHEMA["messages"]["Request"]["fields"]["observe"].is_object() || SCHEMA["messages"]["Request"]["fields"]["movement"].is_object() || SCHEMA["messages"]["Request"]["fields"]["publish"].is_object() {
            let golden:Value=serde_json::from_str(GOLDENS).unwrap();
            let mut value=golden["fixtures"].as_array().unwrap().iter().find(|f|f["type"]=="Request").unwrap()["json"].clone();
            if SCHEMA["messages"]["Request"]["fields"]["observe"].is_object() || SCHEMA["messages"]["Request"]["fields"]["commit"].is_object(){value["serviceId"]=serde_json::json!("fixture");}return value;
        }
        let mut base=serde_json::json!({"major":1,"revision":SCHEMA["revision"],"serviceId":"fixture","requestId":"request-1"});
        if SCHEMA["messages"]["Request"]["fields"]["guestIssue"].is_object() {
            base["guestIssue"]=serde_json::json!({"deviceId":"device-1","displayName":"Guest","issuanceProof":"x".repeat(43)});
        } else { base["pairBegin"]=serde_json::json!({"deviceId":"device-1"}); }
        base
    }
    fn command() -> &'static str {
        if SCHEMA["messages"]["Request"]["fields"]["commit"].is_object() {return "read";}
        if SCHEMA["messages"]["Request"]["fields"]["observe"].is_object() {return "discover";}
        if SCHEMA["messages"]["Request"]["fields"]["publish"].is_object() {return "publish";}
        if SCHEMA["messages"]["Request"]["fields"]["movement"].is_object() {return "movement";}
        if SCHEMA["messages"]["Request"]["fields"]["guestIssue"].is_object() {"guestIssue"} else {"pairBegin"}
    }
    fn read(value: Value) -> Result<wire::Request, ProtocolError> {
        request(&serde_json::to_vec(&value).unwrap())
    }
    #[test]
    fn canonical_dtos_compile_and_decode() {
        let r = read(base()).unwrap();
        let value=serde_json::to_value(r).unwrap();
        if command()=="read" {assert!(value["read"]["locator"].is_string());}else if command()=="movement" {assert_eq!(value["movement"]["frames"][0]["sequence"],1);} else if command()=="discover" {assert!(value["discover"].is_object());} else if command()=="publish" {assert!(value["publish"]["eventId"].is_string());} else {assert_eq!(value[command()]["deviceId"], "device-1");}
    }
    #[test]
    fn reject_unknown_multiple_and_absent_commands() {
        let mut v = base();
        v["unknown"] = true.into();
        assert!(read(v).is_err());
        let mut v = base();
        v["logout"] = serde_json::json!({});
        assert!(read(v).is_err());
        let mut v = base();
        v.as_object_mut().unwrap().remove(command());
        assert!(read(v).is_err());
    }
    #[test]
    fn canonical_byte_count_and_type_limits() {
        let mut v = base();
        v[command()]["deviceId"] = "é".repeat(65).into();
        assert!(read(v).is_err());
        let mut v = base();
        v["major"] = 4294967296u64.into();
        assert!(read(v).is_err());
        let mut v = base();
        v.as_object_mut().unwrap().remove(command());
        v["start"] = serde_json::json!({"participantDesktopIds":vec!["id";33]});
        assert!(read(v).is_err());
        assert!(matches!(
            request(&vec![b' '; SCHEMA["limits"]["bytes"].as_u64().unwrap() as usize+1]),
            Err(ProtocolError::TooLarge)
        ));
    }
    #[test]
    fn reject_missing_identity_and_wrong_revision() {
        let mut v = base();
        v["requestId"] = "".into();
        assert!(read(v).is_err());
        let mut v = base();
        v["revision"] = "other".into();
        assert!(matches!(read(v), Err(ProtocolError::VersionMismatch)));
    }
}
