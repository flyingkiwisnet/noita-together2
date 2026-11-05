use super::*;
mod fixture{include!("c1_startup_fixture.rs");}
struct UntouchedCredentials;
impl crate::secrets::CredentialStore for UntouchedCredentials{
 fn save(&self,_:&crate::secrets::ServiceProfile,_:&crate::secrets::SecretBlob)->Result<(),crate::secrets::StoreError>{panic!("startup must not write credentials")}
 fn load(&self,_:&crate::secrets::ServiceProfile)->Result<Option<crate::secrets::SecretBlob>,crate::secrets::StoreError>{panic!("startup must not read credentials")}
 fn delete(&self,_:&crate::secrets::ServiceProfile)->Result<(),crate::secrets::StoreError>{panic!("startup must not delete credentials")}
}
struct Owned(std::path::PathBuf);
impl Owned{fn new()->Self{let p=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(3).unwrap().join("spikes/noita-entry/.build").join(format!("c1-startup-unit-{}",uuid::Uuid::new_v4()));std::fs::create_dir(&p).unwrap();Self(p)}}
impl Drop for Owned{fn drop(&mut self){let _=std::fs::remove_dir_all(&self.0);}}
fn initialize(root:&std::path::Path)->Result<Box<Core<UntouchedCredentials>>,StartupError>{
 initialize_core(UntouchedCredentials,uuid::Uuid::new_v4(),Arc::new(Mutex::new(None)),root,Arc::new(Mutex::new(None)),Arc::new(Mutex::new(Default::default())),Arc::new(Mutex::new(Default::default())),Arc::new(Mutex::new(Default::default())),true,false)
}
#[test]fn startup_normal_core_initialization_loads_mixed_history_without_profile_or_network_work(){
 let root=Owned::new();let bytes=fixture::mixed_history();let path=root.0.join("g3-operations.jsonl");std::fs::write(&path,&bytes).unwrap();
 let core=initialize(&root.0).unwrap();assert!(core.lc1_enabled());let view=core.view();assert!(!view.is_null());drop(core);
 assert_eq!(bytes,std::fs::read(&path).unwrap());
 let journal=nt2_desktop_g0::g3_journal::Journal::open(&path).unwrap();assert_eq!(journal.pending().count(),13);assert!(journal.pending().all(|p|p.restored));
 assert!(!root.0.join("device-id").exists());
}
#[test]fn startup_normal_core_failure_has_bounded_actionable_diagnostic_and_preserves_history(){
 let root=Owned::new();let path=root.0.join("g3-operations.jsonl");let bytes=b"{synthetic-invalid-history}\n";std::fs::write(&path,bytes).unwrap();
 let error=initialize(&root.0).err().expect("corrupt history must stop initialization");assert_eq!(error.0,"bank-history");
 let mut displayed=None;present_startup_failure(Some(&root.0),error.0,|message|displayed=Some(message.to_owned()));let message=displayed.expect("startup failure must be presented");
 assert!(message.contains("preserved")&&message.contains("coordinator")&&message.contains("Do not delete"));
 let diagnostic=std::fs::read(root.0.join("startup-error.json")).unwrap();assert!(diagnostic.len()<256);let value:Value=serde_json::from_slice(&diagnostic).unwrap();assert_eq!(value["stage"],"bank-history");assert!(!message.contains("synthetic-invalid"));assert_eq!(bytes,std::fs::read(&path).unwrap().as_slice());
 let message=startup_diagnostic(Some(&path),"untrusted data");assert!(message.contains("could not be saved"));assert!(!message.contains("untrusted data"));
}

#[test]fn startup_stack_measure_scheduler_and_nested_futures(){
 fn returned_bytes<F:FnOnce()->T,T>(_:F)->usize{std::mem::size_of::<T>()}
 let root=Owned::new();let mut core=initialize(&root.0).unwrap();
 let core_bytes=std::mem::size_of_val(&*core);
 let tick_bytes=returned_bytes(||core.tick());
 let bank_bytes=returned_bytes(||core.bank_scheduler_tick());
 let scheduler_bytes=returned_bytes(||scheduler::new_boxed(core,Arc::new(AtomicBool::new(false))));
 assert!(scheduler_bytes<64*1024,"Keep actor construction below64KiB on the1MiB production main stack: {scheduler_bytes}");
 eprintln!("STACK_MEASUREMENT core={core_bytes} tick={tick_bytes} bank={bank_bytes} scheduler_result={scheduler_bytes}");
}

#[test]#[ignore="Owned subprocess only"]
fn startup_stack_child_normal_main_and_scheduler(){
 if std::env::var("NT2_STACK_CHILD").as_deref()!=Ok("1"){return}
 std::thread::Builder::new().name("startup-main-1m".into()).stack_size(1024*1024).spawn(||{
  eprintln!("STACK_STAGE main-enter");
  let root=Owned::new();let bytes=fixture::mixed_history();std::fs::write(root.0.join("g3-operations.jsonl"),&bytes).unwrap();
  eprintln!("STACK_STAGE before-initialize");let core=initialize(&root.0).unwrap();eprintln!("STACK_STAGE initialized");
  let stop=Arc::new(AtomicBool::new(false));let(handle,task)=start_scheduler(core,stop.clone());eprintln!("STACK_STAGE actor-created-and-spawned");
  let runtime=tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_all().build().unwrap();
  runtime.block_on(async move{
   eprintln!("STACK_STAGE production-runtime-active");
   let view=tokio::time::timeout(std::time::Duration::from_secs(4),handle.command(|core|Box::pin(async move{Ok(core.view())}))).await.unwrap().unwrap();assert!(!view.is_null());
   tokio::time::sleep(std::time::Duration::from_millis(450)).await;eprintln!("STACK_STAGE actor-polled");stop.store(true,Ordering::Release);
   tokio::time::timeout(std::time::Duration::from_secs(5),task).await.unwrap().unwrap();eprintln!("STACK_STAGE actor-stopped");
  });
  assert_eq!(bytes,std::fs::read(root.0.join("g3-operations.jsonl")).unwrap());
 }).unwrap().join().unwrap();
}
#[test]fn startup_stack_normal_main_and_scheduler_subprocess(){
 use std::process::{Command,Stdio};
 let mut command=Command::new(std::env::current_exe().unwrap());
 command.args(["--exact","next_app::startup_tests::startup_stack_child_normal_main_and_scheduler","--ignored","--nocapture"]).env("NT2_STACK_CHILD","1").stdin(Stdio::null());
 #[cfg(windows)]{use std::os::windows::process::CommandExt;command.creation_flags(0x08000000);}
 let output=command.output().unwrap();eprintln!("{}",String::from_utf8_lossy(&output.stderr));assert!(output.status.success(),"isolated normal-stack startup failed: {:?}",output.status.code());
}
