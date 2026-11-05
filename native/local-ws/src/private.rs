//! Narrow app-lifetime capability sidecar; never a save, journal or capture.
use crate::protocol::Descriptor;
use std::{path::{Path,PathBuf},io::{Read,Write}};
use zeroize::{Zeroize,Zeroizing};
pub const DIRECTORY:&str=".nt2-local";
pub const FILE:&str="bridge.json";
/// Resolve the mod root obtained from the broker's own loaded module path.
/// The mod mount may be a junction (the approved local testing installation);
/// the private directory and descriptor themselves must never be redirected.
pub fn module_descriptor(path:&Path)->Result<PathBuf,String>{
 if !path.is_absolute()||path.components().any(|c|matches!(c,std::path::Component::ParentDir)){return Err("Absolute module bootstrap path required".into())}
 #[cfg(windows)]if !matches!(path.components().next(),Some(std::path::Component::Prefix(p)) if matches!(p.kind(),std::path::Prefix::Disk(_)|std::path::Prefix::VerbatimDisk(_))){return Err("Remote module bootstrap refused".into())}
 if path.file_name().and_then(|p|p.to_str())!=Some(FILE)||path.parent().and_then(|p|p.file_name()).and_then(|p|p.to_str())!=Some(DIRECTORY){return Err("Exact module bootstrap path required".into())}
 let root=path.parent().and_then(|p|p.parent()).ok_or("Module directory unavailable")?.canonicalize().map_err(|_|"Module directory unavailable")?;plain(&root)?;Ok(root.join(DIRECTORY).join(FILE))
}
pub fn plain(path:&Path)->Result<(),String>{
 if !path.is_absolute(){return Err("Absolute local bootstrap path required".into())}
 if path.components().any(|c|matches!(c,std::path::Component::ParentDir)){return Err("Bootstrap parent traversal refused".into())}
 #[cfg(windows)]if !matches!(path.components().next(),Some(std::path::Component::Prefix(p)) if matches!(p.kind(),std::path::Prefix::Disk(_)|std::path::Prefix::VerbatimDisk(_))){return Err("Remote bootstrap path refused".into())}
 for p in path.ancestors(){if let Ok(m)=std::fs::symlink_metadata(p){if m.file_type().is_symlink(){return Err("Redirected bootstrap path refused".into())}#[cfg(windows)]{use std::os::windows::fs::MetadataExt;if m.file_attributes()&0x400!=0{return Err("Redirected bootstrap path refused".into())}}}}
 Ok(())
}
#[cfg(windows)]mod os{
 use super::*;use std::{ffi::c_void,ptr::null_mut};
 type Handle=*mut c_void;
 #[link(name="advapi32")]unsafe extern "system"{
  fn OpenProcessToken(process:Handle,access:u32,token:*mut Handle)->i32;
  fn GetTokenInformation(token:Handle,class:u32,data:Handle,size:u32,needed:*mut u32)->i32;
  fn ConvertSidToStringSidW(sid:Handle,text:*mut *mut u16)->i32;
  fn ConvertStringSecurityDescriptorToSecurityDescriptorW(text:*const u16,revision:u32,descriptor:*mut Handle,size:*mut u32)->i32;
  fn SetFileSecurityW(path:*const u16,info:u32,descriptor:Handle)->i32;
  fn GetNamedSecurityInfoW(path:*const u16,kind:u32,info:u32,owner:*mut Handle,group:*mut Handle,dacl:*mut Handle,sacl:*mut Handle,descriptor:*mut Handle)->u32;
  fn GetSecurityDescriptorControl(sd:Handle,control:*mut u16,revision:*mut u32)->i32;
  fn GetAclInformation(acl:Handle,info:Handle,size:u32,class:u32)->i32;
  fn GetAce(acl:Handle,index:u32,ace:*mut Handle)->i32;
  fn EqualSid(a:Handle,b:Handle)->i32;
 }
 #[link(name="kernel32")]unsafe extern "system"{fn GetCurrentProcess()->Handle;fn CloseHandle(h:Handle)->i32;fn LocalFree(p:Handle)->Handle;}
 fn wide(path:&Path)->Vec<u16>{path.as_os_str().to_string_lossy().encode_utf16().chain(Some(0)).collect()}
 fn user()->Result<Vec<u8>,String>{unsafe{let mut token=null_mut();if OpenProcessToken(GetCurrentProcess(),8,&mut token)==0{return Err("Local capability identity unavailable".into())}let mut n=0;GetTokenInformation(token,1,null_mut(),0,&mut n);let mut b=vec![0;n as usize];let ok=GetTokenInformation(token,1,b.as_mut_ptr().cast(),n,&mut n);CloseHandle(token);if ok==0||n< std::mem::size_of::<Handle>()as u32{return Err("Local capability identity unavailable".into())}Ok(b)}}
 pub fn protect(path:&Path,directory:bool)->Result<(),String>{unsafe{let user=user()?;let sid=*(user.as_ptr().cast::<Handle>());let mut text=null_mut();if ConvertSidToStringSidW(sid,&mut text)==0{return Err("Local capability ACL unavailable".into())}let mut n=0;while *text.add(n)!=0{n+=1;}let sid=String::from_utf16_lossy(std::slice::from_raw_parts(text,n));LocalFree(text.cast());let sddl=format!("D:P(A;{};FA;;;{sid})",if directory{"OICI"}else{""});let encoded:Vec<u16>=sddl.encode_utf16().chain(Some(0)).collect();let mut sd=null_mut();if ConvertStringSecurityDescriptorToSecurityDescriptorW(encoded.as_ptr(),1,&mut sd,null_mut())==0{return Err("Local capability ACL unavailable".into())}let ok=SetFileSecurityW(wide(path).as_ptr(),0x80000004,sd);LocalFree(sd);if ok==0{return Err("Private local capability permissions required".into())}verify(path)}}
 pub fn verify(path:&Path)->Result<(),String>{unsafe{
  let user=user()?;let sid=*(user.as_ptr().cast::<Handle>());let(mut owner,mut acl,mut sd)=(null_mut(),null_mut(),null_mut());
  if GetNamedSecurityInfoW(wide(path).as_ptr(),1,5,&mut owner,null_mut(),&mut acl,null_mut(),&mut sd)!=0{return Err("Private local capability permissions unavailable".into())}
  let result=(||{let(mut control,mut revision)=(0u16,0u32);if acl.is_null()||owner.is_null()||EqualSid(owner,sid)==0||GetSecurityDescriptorControl(sd,&mut control,&mut revision)==0||control&0x1000==0{return false}
   let mut info=[0u32;3];if GetAclInformation(acl,info.as_mut_ptr().cast(),12,2)==0||info[0]!=1{return false}let mut ace=null_mut();if GetAce(acl,0,&mut ace)==0{return false}let a=ace.cast::<u8>();*a==0&&u32::from_le_bytes(std::slice::from_raw_parts(a.add(4),4).try_into().unwrap())&0x1f01ff==0x1f01ff&&EqualSid(a.add(8).cast(),sid)!=0})();LocalFree(sd);if !result{return Err("User-only local capability ACL required".into())}Ok(())
 }}
}
#[cfg(unix)]mod os{
 use super::*;use std::os::unix::fs::{PermissionsExt,MetadataExt};
 pub fn protect(path:&Path,directory:bool)->Result<(),String>{std::fs::set_permissions(path,std::fs::Permissions::from_mode(if directory{0o700}else{0o600})).map_err(|_|"Private local capability permissions required")?;verify(path)}
 pub fn verify(path:&Path)->Result<(),String>{let m=std::fs::symlink_metadata(path).map_err(|_|"Private local capability unavailable")?;let own=std::fs::metadata("/proc/self").map_err(|_|"Local capability identity unavailable")?;if m.uid()!=own.uid()||m.permissions().mode()&0o077!=0{return Err("User-only local capability permissions required".into())}Ok(())}
}
pub struct Bootstrap{path:PathBuf,app:String}
impl Bootstrap{
 pub fn publish(mod_root:&Path,descriptor:&Descriptor)->Result<Self,String>{
  plain(mod_root)?;if !mod_root.is_dir(){return Err("Verified mod root unavailable".into())}let dir=mod_root.join(DIRECTORY);plain(&dir)?;
  if !dir.exists(){std::fs::create_dir(&dir).map_err(|_|"Private local capability directory unavailable")?;os::protect(&dir,true)?;}else{if !dir.is_dir(){return Err("Private capability directory required".into())}os::verify(&dir)?;}
  let path=dir.join(FILE);plain(&path)?;if path.exists(){os::verify(&path)?;}
  let temporary=dir.join(format!(".bridge-{}.tmp",crate::protocol::hex(&crate::protocol::random::<16>()?)));let mut options=std::fs::OpenOptions::new();options.write(true).create_new(true);
  #[cfg(unix)]{use std::os::unix::fs::OpenOptionsExt;options.mode(0o600);}
  let mut file=options.open(&temporary).map_err(|_|"Private capability create failed")?;
  let result=(||{os::protect(&temporary,false)?;let bytes=Zeroizing::new(serde_json::to_vec(descriptor).map_err(|_|"Private capability encode failed")?);file.write_all(&bytes).map_err(|_|"Private capability write failed")?;file.sync_all().map_err(|_|"Private capability persist failed")?;drop(file);std::fs::rename(&temporary,&path).map_err(|_|"Private capability publish failed")?;os::verify(&path)})();if result.is_err(){let _=std::fs::remove_file(&temporary);}result?;Ok(Self{path,app:descriptor.app.clone()})
 }
 pub fn path(&self)->&Path{&self.path}
}
impl Drop for Bootstrap{fn drop(&mut self){if read(&self.path).is_ok_and(|d|d.app==self.app){let _=std::fs::remove_file(&self.path);if let Some(dir)=self.path.parent(){let _=std::fs::remove_dir(dir);}}}}
pub fn read(path:&Path)->Result<Descriptor,String>{
 plain(path)?;if path.file_name().and_then(|s|s.to_str())!=Some(FILE)||path.parent().and_then(|p|p.file_name()).and_then(|s|s.to_str())!=Some(DIRECTORY){return Err("Exact private capability path required".into())}
 os::verify(path.parent().unwrap())?;os::verify(path)?;
 let mut options=std::fs::OpenOptions::new();options.read(true);#[cfg(windows)]{use std::os::windows::fs::OpenOptionsExt;options.custom_flags(0x00200000);}
 let file=options.open(path).map_err(|_|"Private capability unavailable")?;let metadata=file.metadata().map_err(|_|"Private capability unavailable")?;if !metadata.is_file()||metadata.len()>1024{return Err("Private capability bounds".into())}
 let mut b=Zeroizing::new(Vec::new());file.take(1025).read_to_end(&mut b).map_err(|_|"Private capability read failed")?;if b.len()>1024{return Err("Private capability bounds".into())}serde_json::from_slice(&b).map_err(|_|"Invalid private capability".to_string())
}
impl Drop for Descriptor{fn drop(&mut self){self.secret.zeroize()}}
