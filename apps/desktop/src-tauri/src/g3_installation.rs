//! Exact source-reviewed DEVELOPMENT verifier. No installation/launch permission.
//! Immutable runtime approval is separate from frozen desktop artifact provenance.
use sha2::{Digest,Sha256};
use serde_json::Value;
use std::path::{Path,Component};
#[cfg(not(any(feature="g3-a1-runtime-fix",feature="g3-a1-runtime-diagnostic",feature="g3-a1-heartbeat-busy")))]
pub const PROFILE_SHA:&str="959fddf3a0d5948709e685fdfda325de96004d98b798e377b70f65b5ad8502dd";
#[cfg(not(any(feature="g3-a1-runtime-fix",feature="g3-a1-runtime-diagnostic",feature="g3-a1-heartbeat-busy")))]
pub const PROFILE_BYTES:&[u8]=b"{}";
#[cfg(not(any(feature="g3-a1-runtime-fix",feature="g3-a1-runtime-diagnostic",feature="g3-a1-heartbeat-busy")))]
pub const PROFILE_ID:&str="nt2-g3-native-development-3";
#[cfg(all(feature="g3-a1-runtime-fix",not(any(feature="g3-a1-runtime-diagnostic",feature="g3-a1-heartbeat-busy"))))]
pub const PROFILE_SHA:&str="301b1a8bd0d88804303ea7d9a0d199c71a014ae38fe6c8528704b862a76f1eb8";
#[cfg(all(feature="g3-a1-runtime-fix",not(any(feature="g3-a1-runtime-diagnostic",feature="g3-a1-heartbeat-busy"))))]
pub const PROFILE_BYTES:&[u8]=b"{}";
#[cfg(all(feature="g3-a1-runtime-fix",not(any(feature="g3-a1-runtime-diagnostic",feature="g3-a1-heartbeat-busy"))))]
pub const PROFILE_ID:&str="nt2-g3-native-development-4";
#[cfg(all(feature="g3-a1-runtime-diagnostic",not(feature="g3-a1-heartbeat-busy")))]
pub const PROFILE_SHA:&str="54ffd19ba95f4a278a59bdf306725415cbd806072361c7c3918991dbcfa6c042";
#[cfg(all(feature="g3-a1-runtime-diagnostic",not(feature="g3-a1-heartbeat-busy")))]
pub const PROFILE_BYTES:&[u8]=b"{}";
#[cfg(all(feature="g3-a1-runtime-diagnostic",not(feature="g3-a1-heartbeat-busy")))]
pub const PROFILE_ID:&str="nt2-g3-native-development-5";
#[cfg(feature="g3-a1-heartbeat-busy")]
pub const PROFILE_SHA:&str="f43a9d00836d92f4e0b37c06066d7f85e95e24e792a099fa6c545d95788142c0";
#[cfg(feature="g3-a1-heartbeat-busy")]
pub const PROFILE_BYTES:&[u8]=b"{}";
#[cfg(feature="g3-a1-heartbeat-busy")]
pub const PROFILE_ID:&str="nt2-g3-native-development-6";
const DESKTOP_REVIEW:&[u8]=b"{}";
fn hex(bytes:&[u8])->String{bytes.iter().map(|b|format!("{b:02x}")).collect()}
pub fn profile()->Result<Value,String>{if hex(&Sha256::digest(PROFILE_BYTES))!=PROFILE_SHA{return Err("G3 compiled profile drift".into());}{let built:Value=serde_json::from_slice(PROFILE_BYTES).map_err(|_|"Invalid G3 development profile")?;if built["id"]!=PROFILE_ID||built["serviceId"]!="nt2-g3-native-development-a1"||built["namespacePrefix"]!="g3_a1_"||built["sourceApproved"]!=true||built["developmentOnly"]!=true||built["installApproved"]!=false||built["packaged"]!=false||built["actualNoitaEvidence"]!=false||built["compatibility"]["capabilities"]!=serde_json::json!(["core.seed","core.ghost"]){return Err("G3 source-only policy refused".into());}Ok(built)}}
pub fn pair(metadata:&Value,exe:&str,lua:&str)->Result<String,String>{
 let built=profile()?;if metadata!=&built["profile"]{return Err("Service G3 profile differs from compiled exact review".into());}
 built["profile"]["supportedBuilds"].as_array().ok_or("Missing exact build pairs")?.iter().find(|p|p["gameExeSha256"]==exe&&p["originalLuaSha256"]==lua)
  .and_then(|p|p["id"].as_str()).map(str::to_owned).ok_or("Unsupported exact G3 executable/original Lua pair".into())
}
pub(crate) fn local(path:&Path)->Result<(),String>{
 if !path.is_absolute(){return Err("Local absolute artifact path required".into());}
 let mut current=std::path::PathBuf::new();for part in path.components(){if matches!(part,Component::ParentDir){return Err("Artifact parent traversal refused".into());}
  if let Component::Prefix(prefix)=part{if !matches!(prefix.kind(),std::path::Prefix::Disk(_)|std::path::Prefix::VerbatimDisk(_)){return Err("Nonlocal artifact path refused".into());}}
  current.push(part);
  if let Ok(meta)=std::fs::symlink_metadata(&current){use std::os::windows::fs::MetadataExt;if meta.file_attributes()&0x400!=0{return Err("Artifact reparse point refused".into());}}
 }Ok(())
}
pub(crate) fn hash(path:&Path)->Result<String,String>{
 use std::io::Read;local(path)?;let mut file=std::fs::File::open(path).map_err(|_|"G3 artifact unavailable")?;
 if file.metadata().map_err(|_|"G3 artifact metadata unavailable")?.len()>256*1024*1024{return Err("G3 artifact capacity".into());}
 let mut digest=Sha256::new();let mut bytes=[0u8;65536];let mut total=0usize;
 loop{let count=file.read(&mut bytes).map_err(|_|"G3 artifact interrupted")?;if count==0{break;}total+=count;if total>256*1024*1024{return Err("G3 artifact capacity".into());}digest.update(&bytes[..count]);}Ok(hex(&digest.finalize()))
}
/// Hash installed game/native/mod bytes against the exact reviewed dependency set.
pub fn verify(root:&Path,metadata:&Value)->Result<String,String>{
 verify_using(root,metadata,hash)
}
/// A review artifact reference, not a circular self-hash requirement for a new build.
pub fn verify_frozen_desktop(desktop:&Path)->Result<(),String>{
 if hex(&Sha256::digest(DESKTOP_REVIEW))!="ccfc578524292620af02b5377751304bbd95873edbbc6fa66dbdfbef8894b1b9"{return Err("Frozen artifact review drift".into());}
 let built:Value=serde_json::from_slice(DESKTOP_REVIEW).map_err(|_|"Invalid frozen artifact review")?;
 let expected=built["pins"].as_array().ok_or("Missing frozen pins")?.iter().find(|p|p["path"].as_str().is_some_and(|p|p.ends_with("/g3_candidate.exe"))).ok_or("Missing frozen desktop")?;
 if Some(hash(desktop)?.as_str())!=expected["sha256"].as_str(){return Err("Changed frozen desktop reference".into());}Ok(())
}
fn verify_using(root:&Path,metadata:&Value,read:impl Fn(&Path)->Result<String,String>)->Result<String,String>{
 let built=profile()?;let label=pair(metadata,&read(&root.join("noita.exe"))?,&read(&root.join("lua51.dll"))?)?;
 let files=built["requiredLocalFiles"].as_array().ok_or("Missing exact local runtime mapping")?;
 if files.len()!=14{return Err("Incomplete fourteen-asset runtime closure".into());}
 let mut seen=std::collections::BTreeSet::new();
 for file in files{
  let path=file["path"].as_str().ok_or("Invalid local runtime mapping")?;
  if !path.starts_with("mods/noita_together2/")||path.contains('\\')||path.split('/').any(|p|p.is_empty()||p=="."||p==".."||p.contains(':'))||!seen.insert(path){return Err("Unsafe/duplicate local runtime mapping".into());}
  let target=root.join(path);if read(&target)?.as_str()!=file["sha256"].as_str().ok_or("Invalid runtime asset hash")?{return Err(format!("Changed G3 required artifact: {}",target.display()));}
 }
 Ok(label)
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn real_owned_runtime_bytes_each_tamper_and_frozen_desktop_reference(){
  let built=profile().unwrap();let repo=Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();
  let artifact:Value=serde_json::from_slice(DESKTOP_REVIEW).unwrap();verify_frozen_desktop(&repo.join(artifact["candidateExecutable"].as_str().unwrap())).unwrap();
  let root=repo.join("spikes/noita-entry/.build/g3-verifier").join(uuid::Uuid::new_v4().to_string());
  std::fs::create_dir_all(&root).unwrap();
  let mut assets=Vec::new();
  for file in built["requiredLocalFiles"].as_array().unwrap(){
   let pin=built["provenance"]["pins"].as_array().unwrap().iter().find(|p|p["sha256"]==file["sha256"]).unwrap();
   let target=root.join(file["path"].as_str().unwrap());std::fs::create_dir_all(target.parent().unwrap()).unwrap();let bytes=std::fs::read(repo.join(pin["path"].as_str().unwrap())).unwrap();std::fs::write(&target,&bytes).unwrap();assets.push((target,bytes));
  }
  // Only executable/original-runtime fingerprints are injected. All candidate assets are real copied bytes.
  for pair in built["profile"]["supportedBuilds"].as_array().unwrap(){
   let read=|p:&Path|if p==root.join("noita.exe"){Ok(pair["gameExeSha256"].as_str().unwrap().to_owned())}else if p==root.join("lua51.dll"){Ok(pair["originalLuaSha256"].as_str().unwrap().to_owned())}else{hash(p)};
   assert_eq!(verify_using(&root,&built["profile"],read).unwrap(),pair["id"].as_str().unwrap());
   for (target,bytes) in &assets{let mut changed=bytes.clone();changed[0]^=1;std::fs::write(target,changed).unwrap();assert!(verify_using(&root,&built["profile"],read).is_err());std::fs::remove_file(target).unwrap();assert!(verify_using(&root,&built["profile"],read).is_err());std::fs::write(target,bytes).unwrap();}
  }
  let bad=root.join("unreviewed-desktop.exe");std::fs::write(&bad,b"unreviewed").unwrap();assert!(verify_frozen_desktop(&bad).is_err());
  assert!(hash(Path::new(r"\\host\share\file")).is_err());assert!(hash(&root.join("../outside")).is_err());
  // No installed files, saves, or original executable/runtime bytes are read or changed.
 }
 #[test]fn exact_reviewed_pairs_metadata_capabilities_and_each_runtime_dependency(){
  let built=profile().unwrap();assert_eq!(built["id"],PROFILE_ID);assert_eq!(built["compatibility"]["capabilities"],serde_json::json!(["core.seed","core.ghost"]));
  // Source/check evidence may live under apps/desktop; a runtime resource must
  // not depend on the future desktop executable's own digest.
  assert!(built["provenance"]["pins"].as_array().unwrap().iter().all(|p|!p["path"].as_str().unwrap().ends_with(".exe")));
  let root=Path::new(r"C:\nt2-owned-verifier-fixture");
  for pair in built["profile"]["supportedBuilds"].as_array().unwrap(){
   let mut values=std::collections::BTreeMap::new();values.insert(root.join("noita.exe"),pair["gameExeSha256"].as_str().unwrap().to_owned());values.insert(root.join("lua51.dll"),pair["originalLuaSha256"].as_str().unwrap().to_owned());
   for file in built["requiredLocalFiles"].as_array().unwrap(){values.insert(root.join(file["path"].as_str().unwrap()),file["sha256"].as_str().unwrap().into());}
   let read=|p:&Path|values.get(p).cloned().ok_or("Injected absent file".into());assert_eq!(verify_using(root,&built["profile"],read).unwrap(),pair["id"].as_str().unwrap());
   for target in values.keys(){let read=|p:&Path|if p==target{Ok("0".repeat(64))}else{values.get(p).cloned().ok_or("Injected absent file".into())};assert!(verify_using(root,&built["profile"],read).is_err(),"{}",target.display());}
  }
  let mut forged=built["profile"].clone();forged["bridgeSha256"]=serde_json::json!("0".repeat(64));assert!(pair(&forged,""," ").is_err());
 }
}
