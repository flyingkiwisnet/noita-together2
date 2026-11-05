// Private composition-file transport only; never a gameplay original retry.
use std::{io,path::Path,time::{Duration,Instant}};
fn transient(e:&io::Error)->bool{matches!(e.raw_os_error(),Some(5|32|33))}
fn retry<T>(budget:Duration,mut operation:impl FnMut()->io::Result<T>)->io::Result<T>{
 let until=Instant::now()+budget;loop{match operation(){Err(e)if transient(&e)&&Instant::now()<until=>std::thread::sleep(Duration::from_millis(2)),result=>return result}}
}
const BUDGET:Duration=Duration::from_millis(250);
pub(super) fn write(path:impl AsRef<Path>,bytes:impl AsRef<[u8]>)->io::Result<()>{
 let path=path.as_ref();let parent=path.parent().ok_or_else(||io::Error::new(io::ErrorKind::InvalidInput,"Signal parent absent"))?;
 if path.is_dir(){return Err(io::Error::new(io::ErrorKind::InvalidInput,"Signal target is a directory"))}
 // Readers never see a truncated control value. A retry publishes the SAME
 // completed file, and stops immediately on successful publication.
 let temp=parent.join(format!(".a3-{}.tmp",uuid::Uuid::new_v4().simple()));
 let result=(||{retry(BUDGET,||std::fs::write(&temp,bytes.as_ref()))?;retry(BUDGET,||std::fs::rename(&temp,path))})();
 if result.is_err(){let _=remove(&temp);}result
}
pub(super) fn remove(path:impl AsRef<Path>)->io::Result<()>{retry(BUDGET,||match std::fs::remove_file(path.as_ref()){Err(e)if e.kind()==io::ErrorKind::NotFound=>Ok(()),r=>r})}
pub(super) fn read(path:impl AsRef<Path>)->io::Result<Option<String>>{retry(BUDGET,||match std::fs::read_to_string(path.as_ref()){Ok(v)=>Ok(Some(v)),Err(e)if e.kind()==io::ErrorKind::NotFound=>Ok(None),Err(e)=>Err(e)})}
pub(super) fn present(path:impl AsRef<Path>)->io::Result<bool>{retry(BUDGET,||match std::fs::metadata(path.as_ref()){Ok(m)if m.is_file()=>Ok(true),Ok(_)=>Err(io::Error::new(io::ErrorKind::InvalidInput,"Signal target is not a file")),Err(e)if e.kind()==io::ErrorKind::NotFound=>Ok(false),Err(e)=>Err(e)})}
#[test]fn a3_fixture_signal_retry_is_bounded_and_semantic_errors_surface(){
 let mut attempts=0;let start=Instant::now();let result=retry(Duration::from_millis(12),||{attempts+=1;Err::<(),_>(io::Error::from_raw_os_error(5))});assert_eq!(result.unwrap_err().raw_os_error(),Some(5));assert!(attempts>1&&start.elapsed()<Duration::from_secs(1));
 for code in [2,3,87,112]{let mut attempts=0;let error=retry(BUDGET,||{attempts+=1;Err::<(),_>(io::Error::from_raw_os_error(code))}).unwrap_err();assert_eq!(error.raw_os_error(),Some(code));assert_eq!(attempts,1,"A semantic error was retried");}
}
#[cfg(windows)]#[test]fn a3_fixture_signal_real_windows_reader_contention_keeps_one_complete_action(){
 use std::os::windows::fs::OpenOptionsExt;
 let root=super::repo().join("spikes/noita-entry/.build").join(format!("a3-signal-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&root).unwrap();let file=root.join("a3-select");std::fs::write(&file,"old").unwrap();
 let hold=std::fs::OpenOptions::new().read(true).share_mode(0).open(&file).unwrap();let release=std::thread::spawn(move||{std::thread::sleep(Duration::from_millis(40));drop(hold)});assert_eq!(read(&file).unwrap().as_deref(),Some("old"));release.join().unwrap();
 let hold=std::fs::OpenOptions::new().read(true).share_mode(3).open(&file).unwrap();let release=std::thread::spawn(move||{std::thread::sleep(Duration::from_millis(40));drop(hold)});write(&file,"color_red").unwrap();release.join().unwrap();assert_eq!(read(&file).unwrap().as_deref(),Some("color_red"));assert_eq!(std::fs::read_dir(&root).unwrap().count(),1,"Publication duplicated the logical action");
 let hold=std::fs::OpenOptions::new().read(true).share_mode(3).open(&file).unwrap();let release=std::thread::spawn(move||{std::thread::sleep(Duration::from_millis(40));drop(hold)});remove(&file).unwrap();release.join().unwrap();assert!(!present(&file).unwrap());remove(&file).unwrap();
 assert!(write(&root,"wrong type").is_err());std::fs::remove_dir(root).unwrap();
}
