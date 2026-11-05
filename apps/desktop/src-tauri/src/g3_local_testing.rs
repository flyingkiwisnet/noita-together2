//! One explicit local correction junction. All other reparse points still refuse.
use std::path::Path;
const TARGET:&str="C:/NT2-Source-Example/testing/current/mod/noita_together2";
pub(crate) fn hash(root:&Path,path:&Path)->Result<String,String>{
 hash_with_target(root,path,Path::new(TARGET))
}
fn hash_with_target(root:&Path,path:&Path,target:&Path)->Result<String,String>{
 let link=root.join("mods/noita_together2");
 let Some(relative)=path.strip_prefix(&link).ok().filter(|p|!p.as_os_str().is_empty()) else{return crate::g3_installation::hash(path)};
 use std::os::windows::fs::MetadataExt;
 let metadata=std::fs::symlink_metadata(&link).map_err(|_|"Local mod folder unavailable")?;
 if metadata.file_attributes()&0x400==0{return crate::g3_installation::hash(path)}
 // Validate every parent without accepting any link in the game root/mods.
 crate::g3_installation::local(link.parent().ok_or("Missing mod parent")?)?;
 crate::g3_installation::local(target)?;
 if !is_junction(&link)?{return Err("Only the exact local testing directory junction is supported".into())}
 let actual=std::fs::read_link(&link).map_err(|_|"Cannot resolve local testing junction")?;
 // Lexical full target check comes before canonicalization: no second link
 // or redirected ancestor can be used to arrive at the intended target.
 if clean(&actual)!=clean(target){return Err("Unknown local testing junction target".into())}
 let resolved=target.join(relative);crate::g3_installation::local(&resolved)?;
 let before=std::fs::canonicalize(&link).map_err(|_|"Local junction target unavailable")?;
 if clean(&before)!=clean(&std::fs::canonicalize(target).map_err(|_|"Testing target unavailable")?){return Err("Local junction resolution mismatch".into())}
 let digest=crate::g3_installation::hash(&resolved)?;
 if std::fs::read_link(&link).map(|p|clean(&p)).ok()!=Some(clean(target)){return Err("Local junction changed during verification".into())}
 Ok(digest)
}
#[cfg(test)]pub(crate) fn test_hash(root:&Path,path:&Path,target:&Path)->Result<String,String>{hash_with_target(root,path,target)}
fn clean(p:&Path)->String{p.to_string_lossy().trim_start_matches(r"\\?\").replace('/',"\\").trim_end_matches('\\').to_ascii_lowercase()}
fn is_junction(path:&Path)->Result<bool,String>{
 use std::os::windows::{fs::OpenOptionsExt,io::AsRawHandle};
 #[repr(C)]struct Attributes{attributes:u32,tag:u32}
 #[link(name="kernel32")]unsafe extern "system"{fn GetFileInformationByHandleEx(handle:*mut std::ffi::c_void,class:i32,data:*mut std::ffi::c_void,size:u32)->i32;}
 let file=std::fs::OpenOptions::new().read(true).custom_flags(0x02000000|0x00200000).open(path).map_err(|_|"Cannot inspect local junction")?;
 let mut data=Attributes{attributes:0,tag:0};
 let ok=unsafe{GetFileInformationByHandleEx(file.as_raw_handle(),9,(&mut data as *mut Attributes).cast(),std::mem::size_of::<Attributes>() as u32)};
 if ok==0{return Err("Local junction tag unavailable".into())}Ok(data.tag==0xa0000003)
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn exact_target_link_only_and_tampered_bytes_remain_visible(){
  let canonical=Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();
  let repo=std::path::PathBuf::from(canonical.to_str().unwrap().trim_start_matches(r"\\?\"));
  let scratch=repo.join("spikes/noita-entry/.build").join(format!("g3-local-link-verifier-{}",uuid::Uuid::new_v4()));
  let game=scratch.join("game");let target=scratch.join("testing/current/mod/noita_together2");
  std::fs::create_dir_all(game.join("mods")).unwrap();std::fs::create_dir_all(&target).unwrap();
  let file=target.join("init.lua");std::fs::write(&file,b"reviewed").unwrap();
  let link=game.join("mods/noita_together2");
  let output=std::process::Command::new("powershell.exe").args(["-NoProfile","-Command","New-Item -ItemType Junction -Path $env:NT2_TEST_LINK -Target $env:NT2_TEST_TARGET | Out-Null"]).env("NT2_TEST_LINK",&link).env("NT2_TEST_TARGET",&target).output().unwrap();assert!(output.status.success(),"{}",String::from_utf8_lossy(&output.stderr));
  let linked=link.join("init.lua");let digest=hash_with_target(&game,&linked,&target).unwrap();assert_eq!(digest,crate::g3_installation::hash(&file).unwrap());
  assert!(hash(&game,&linked).is_err(),"production literal must refuse scratch target");
  assert!(hash_with_target(&game,&linked,&scratch.join("other")).is_err());
  assert!(crate::g3_installation::hash(&linked).is_err(),"default verifier still refuses all links");
  std::fs::write(&file,b"changed").unwrap();assert_ne!(hash_with_target(&game,&linked,&target).unwrap(),digest);
  std::fs::write(game.join("noita.exe"),b"original").unwrap();assert_eq!(hash_with_target(&game,&game.join("noita.exe"),&target).unwrap(),crate::g3_installation::hash(&game.join("noita.exe")).unwrap());
  let nested=target.join("nested");let outside=scratch.join("outside");std::fs::create_dir_all(&outside).unwrap();std::fs::write(outside.join("item.lua"),b"outside").unwrap();
  assert!(std::process::Command::new("powershell.exe").args(["-NoProfile","-Command","New-Item -ItemType Junction -Path $env:NT2_TEST_LINK -Target $env:NT2_TEST_TARGET | Out-Null"]).env("NT2_TEST_LINK",&nested).env("NT2_TEST_TARGET",&outside).status().unwrap().success());
  assert!(hash_with_target(&game,&link.join("nested/item.lua"),&target).is_err(),"nested target links refuse");
 }
}
