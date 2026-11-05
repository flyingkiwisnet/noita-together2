//! Candidate bounded local observation framing, separate from accepted network codecs.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{Duration, Instant};
pub const HEADER_BYTES: usize = 48;
pub const REPORT_BYTES: usize = 32 * 1024;
pub const MOD_LIMIT: usize = 256;
pub const MOD_ID_BYTES: usize = 128;
const MAGIC: u32 = 0x3247544e;
const VERSION: u32 = 3;
const REPORT_OP: u32 = 7;
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Observation {
    pub actual_seed: Option<String>,
    pub mods: Vec<String>,
    pub complete: bool,
    pub error: Option<String>,
    pub beta: Option<bool>,
    pub loaded: bool,
    pub protected: bool,
    pub mod_version: String,
    pub bridge_version: String,
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub race_classic_available:Option<bool>,
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub race_map_available:Option<bool>,
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub race_ng0:Option<bool>,
}
impl Observation {
    /// Local detector facts only. Canonical reports use the existing failed
    /// collector projection; no Race field is added to the service protocol.
    pub fn race_failure(&self,config:&serde_json::Value)->Option<&'static str>{
        if config["mode"]!="race"{return None}
        if self.race_ng0!=Some(true){return Some("Race requires verified NG0 detector support.")}
        for goal in config["race"]["goals"].as_array().into_iter().flatten(){
            if goal["id"]=="classic_ending"&&self.race_classic_available!=Some(true){return Some("Selected Classic Ending detector is unavailable for this game source.")}
            if goal["id"]=="visit_parallel_world"&&self.race_map_available!=Some(true){return Some("Selected Parallel World detector is unavailable for this map.")}
        }None
    }
    pub fn validate(&self) -> Result<(), &'static str> {
        if let Some(seed) = &self.actual_seed {
            if seed.is_empty()
                || seed.len() > 20
                || !seed.bytes().all(|b| b.is_ascii_digit())
                || (seed.len() > 1 && seed.starts_with('0'))
            {
                return Err("Invalid observed seed");
            }
            seed.parse::<u64>().map_err(|_| "Observed seed overflow")?;
        }
        if self.mods.len() > MOD_LIMIT
            || self.mods.iter().any(|id| {
                id.is_empty()
                    || id.len() > MOD_ID_BYTES
                    || id.chars().any(|c| c == '\0' || c.is_control())
            })
        {
            return Err("Invalid active mod inventory");
        }
        if self.complete && (self.actual_seed.is_none() || self.error.is_some()) {
            return Err("Incomplete report claimed complete");
        }
        if self.error.as_ref().is_some_and(|e| {
            !matches!(
                e.as_str(),
                "seed_unavailable"
                    | "mods_unavailable"
                    | "inventory_limit"
                    | "invalid_inventory"
                    | "collection_failed"
                    | "protection_failed"
            )
        }) {
            return Err("Unknown collection failure");
        }
        if [&self.mod_version, &self.bridge_version]
            .iter()
            .any(|v| v.is_empty() || v.len() > 128 || v.chars().any(char::is_control))
        {
            return Err("Invalid declared version");
        }
        Ok(())
    }
    pub fn content_digest(&self) -> Result<[u8; 32], &'static str> {
        Ok(Sha256::digest(self.content_bytes()?).into())
    }
    pub fn content_bytes(&self) -> Result<Vec<u8>, &'static str> {
        self.validate()?;
        // Eligibility phase changes do not alter generation/mod identity.
        let content = serde_json::to_vec(&(
            &self.actual_seed,
            &self.mods,
            self.beta,
            &self.mod_version,
            &self.bridge_version,
        ))
        .map_err(|_| "Report encoding failed")?;
        Ok(content)
    }
}
pub struct Frame {
    pub sequence: u64,
    pub context: [u8; 16],
    pub observation: Observation,
}
pub fn header(bytes: &[u8]) -> Result<(usize, u64, [u8; 16]), &'static str> {
    if bytes.len() != HEADER_BYTES {
        return Err("Invalid report header size");
    }
    let field = |i| u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap());
    if field(0) != MAGIC || field(4) != VERSION || field(8) != REPORT_OP || bytes[40..48] != [0; 8]
    {
        return Err("Unknown report frame");
    }
    let size = field(12) as usize;
    let sequence = u64::from_le_bytes(bytes[16..24].try_into().unwrap());
    let context = bytes[24..40].try_into().unwrap();
    if size == 0 || size > REPORT_BYTES || sequence == 0 || context == [0; 16] {
        return Err("Report frame exceeds bounds");
    }
    Ok((size, sequence, context))
}
pub fn decode(bytes: &[u8]) -> Result<Frame, &'static str> {
    if bytes.len() < HEADER_BYTES {
        return Err("Partial report header");
    }
    let (size, sequence, context) = header(&bytes[..HEADER_BYTES])?;
    if bytes.len() != HEADER_BYTES + size {
        return Err("Partial or trailing report body");
    }
    let observation: Observation =
        serde_json::from_slice(&bytes[HEADER_BYTES..]).map_err(|_| "Malformed report body")?;
    observation.validate()?;
    Ok(Frame {
        sequence,
        context,
        observation,
    })
}
pub fn encode(
    context: [u8; 16],
    sequence: u64,
    observation: &Observation,
) -> Result<Vec<u8>, &'static str> {
    observation.validate()?;
    let body = serde_json::to_vec(observation).map_err(|_| "Report encoding failed")?;
    let mut bytes = vec![0u8; HEADER_BYTES];
    bytes[..4].copy_from_slice(&MAGIC.to_le_bytes());
    bytes[4..8].copy_from_slice(&VERSION.to_le_bytes());
    bytes[8..12].copy_from_slice(&REPORT_OP.to_le_bytes());
    bytes[12..16].copy_from_slice(&(body.len() as u32).to_le_bytes());
    bytes[16..24].copy_from_slice(&sequence.to_le_bytes());
    bytes[24..40].copy_from_slice(&context);
    header(&bytes)?;
    bytes.extend(body);
    Ok(bytes)
}
#[derive(Clone, Debug)]
pub struct Latest {
    pub context: [u8; 16],
    pub sequence: u64,
    pub observation: Observation,
    pub digest: [u8; 32],
    pub observed_at: Instant,
}
impl Latest {
    pub fn update(
        previous: &mut Option<Self>,
        context: [u8; 16],
        frame: Frame,
    ) -> Result<bool, &'static str> {
        if context != frame.context {
            return Err("Wrong world report");
        }
        let digest = frame.observation.content_digest()?;
        if let Some(old) = previous {
            if old.context != context {
                return Err("Replacement world report");
            }
            if frame.sequence < old.sequence {
                return Err("Stale observation sequence");
            }
            if frame.sequence == old.sequence {
                if old.observation != frame.observation {
                    return Err("Conflicting observation sequence");
                }
                return Ok(false); // A retransmission is not a new actual observation.
            }
        }
        *previous = Some(Self {
            context,
            sequence: frame.sequence,
            observation: frame.observation,
            digest,
            observed_at: Instant::now(),
        });
        Ok(true)
    }
    pub fn eligible(&self, expected_seed: &str, age: Duration) -> bool {
        self.observed_at.elapsed() < age
            && self.observation.complete
            && self.observation.loaded
            && self.observation.protected
            && self.observation.actual_seed.as_deref() == Some(expected_seed)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_shared_content_bytes_and_digest() {
        let fixtures: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../packages/contracts/g2-next/content-digest-golden.json"
        )))
        .unwrap();
        for fixture in fixtures["fixtures"].as_array().unwrap() {
            let tuple = &fixture["tuple"];
            let report = Observation {
                actual_seed: serde_json::from_value(tuple[0].clone()).unwrap(),
                mods: serde_json::from_value(tuple[1].clone()).unwrap(),
                beta: serde_json::from_value(tuple[2].clone()).unwrap(),
                mod_version: tuple[3].as_str().unwrap().into(),
                bridge_version: tuple[4].as_str().unwrap().into(),
                complete: false,
                error: Some("collection_failed".into()),
                loaded: true,
                protected: true,
            ..Default::default()};
            let bytes = report.content_bytes().unwrap();
            assert_eq!(
                String::from_utf8(bytes.clone()).unwrap(),
                fixture["canonical"].as_str().unwrap()
            );
            assert_eq!(
                bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
                fixture["utf8Hex"].as_str().unwrap()
            );
            assert_eq!(
                report
                    .content_digest()
                    .unwrap()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>(),
                fixture["sha256"].as_str().unwrap()
            );
        }
    }
    fn observation() -> Observation {
        Observation {
            actual_seed: Some("424242".into()),
            mods: vec!["noita_together2".into(), "other_mod".into()],
            complete: true,
            error: None,
            beta: Some(false),
            loaded: true,
            protected: true,
            mod_version: "nt2-g2-inspect-1".into(),
            bridge_version: "g2-inspect-1".into(),
        ..Default::default()}
    }
    #[test]
    fn equivalent_build_family_keeps_actual_beta_in_frame_and_digest() {
        let mut report = observation();
        let stable = report.content_digest().unwrap();
        report.beta = Some(true);
        let frame = decode(&encode([1; 16], 1, &report).unwrap()).unwrap();
        assert_eq!(frame.observation.beta, Some(true));
        assert_ne!(stable, report.content_digest().unwrap());
        assert!(
            serde_json::to_string(&report)
                .unwrap()
                .contains("\"beta\":true")
        );
    }
    #[test]
    fn complete_ordered_inventory_and_host_wait_facts() {
        let report = observation();
        let bytes = encode([1; 16], 1, &report).unwrap();
        let frame = decode(&bytes).unwrap();
        assert_eq!(frame.observation, report);
        let mut latest = None;
        assert!(Latest::update(&mut latest, [1; 16], frame).unwrap());
        assert!(
            latest
                .as_ref()
                .unwrap()
                .eligible("424242", Duration::from_secs(30))
        );
        let before = latest.as_ref().unwrap().observed_at;
        assert!(!Latest::update(&mut latest, [1; 16], decode(&bytes).unwrap()).unwrap());
        assert_eq!(latest.as_ref().unwrap().observed_at, before);
        let mut changed = report.clone();
        changed.mods.reverse();
        assert_ne!(
            changed.content_digest().unwrap(),
            report.content_digest().unwrap()
        );
        assert!(
            Latest::update(
                &mut latest,
                [1; 16],
                decode(&encode([1; 16], 1, &changed).unwrap()).unwrap()
            )
            .is_err()
        );
        assert!(Latest::update(&mut latest, [2; 16], decode(&bytes).unwrap()).is_err());
        assert!(
            Latest::update(
                &mut latest,
                [1; 16],
                decode(&encode([1; 16], 2, &changed).unwrap()).unwrap()
            )
            .unwrap()
        );
        let mut released = changed.clone();
        released.protected = false;
        assert_eq!(
            released.content_digest().unwrap(),
            changed.content_digest().unwrap()
        );
        assert!(
            !Latest {
                context: [1; 16],
                sequence: 3,
                digest: released.content_digest().unwrap(),
                observation: released,
                observed_at: Instant::now()
            }
            .eligible("424242", Duration::from_secs(30))
        );
    }
    #[test]
    fn hostile_partial_oversize_unknown_seed_and_stale_facts() {
        let mut report = observation();
        let bytes = encode([1; 16], 1, &report).unwrap();
        for n in [0, 47, 48, bytes.len() - 1] {
            assert!(decode(&bytes[..n]).is_err());
        }
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(decode(&trailing).is_err());
        let mut overflow = bytes.clone();
        overflow[12..16].copy_from_slice(&((REPORT_BYTES + 1) as u32).to_le_bytes());
        assert!(decode(&overflow).is_err());
        report.mods = vec!["x".into(); MOD_LIMIT + 1];
        assert!(encode([1; 16], 1, &report).is_err());
        report = observation();
        report.mods[0] = "x".repeat(MOD_ID_BYTES + 1);
        assert!(encode([1; 16], 1, &report).is_err());
        report = observation();
        report.actual_seed = Some("0424242".into());
        assert!(report.validate().is_err());
        report.actual_seed = None;
        assert!(report.validate().is_err());
        report.complete = false;
        report.error = Some("seed_unavailable".into());
        assert!(report.validate().is_ok());
        let mut latest = None;
        Latest::update(
            &mut latest,
            [1; 16],
            decode(&encode([1; 16], 1, &report).unwrap()).unwrap(),
        )
        .unwrap();
        assert!(
            !latest
                .as_ref()
                .unwrap()
                .eligible("424242", Duration::from_secs(30))
        );
        latest.as_mut().unwrap().observed_at = Instant::now() - Duration::from_secs(31);
        assert!(
            !latest
                .as_ref()
                .unwrap()
                .eligible("424242", Duration::from_secs(30))
        );
        let body = br#"{"actualSeed":"1","actualSeed":"2"}"#;
        let mut duplicate = bytes[..HEADER_BYTES].to_vec();
        duplicate[12..16].copy_from_slice(&(body.len() as u32).to_le_bytes());
        duplicate.extend(body);
        assert!(decode(&duplicate).is_err());
    }
}

#[cfg(test)]mod race_detector_tests{
 use super::*;use serde_json::json;
 #[test]fn race_selected_detector_gate_applies_before_first_service_reconciliation(){
  let mut o=Observation{race_ng0:Some(true),race_classic_available:Some(false),race_map_available:Some(false),..Default::default()};
  let mut config=json!({"mode":"race","race":{"goals":[{"id":"reach_max_hp"}]}});assert!(o.race_failure(&config).is_none(),"Unavailable unselected detectors must not block HP");
  config["race"]["goals"]=json!([{"id":"classic_ending"}]);assert!(o.race_failure(&config).unwrap().contains("Classic"));o.race_classic_available=Some(true);assert!(o.race_failure(&config).is_none());
  config["race"]["goals"]=json!([{"id":"visit_parallel_world"}]);assert!(o.race_failure(&config).unwrap().contains("Parallel"));o.race_map_available=Some(true);assert!(o.race_failure(&config).is_none());
  o.race_ng0=None;assert!(o.race_failure(&config).unwrap().contains("NG0"));config["mode"]=json!("core");assert!(o.race_failure(&config).is_none(),"Core collection remains independent");
 }
}
