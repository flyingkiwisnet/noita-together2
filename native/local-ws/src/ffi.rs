use std::sync::OnceLock;
static CLIENT:OnceLock<crate::client::Client>=OnceLock::new();
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nt2_local_ws_counters(output:*mut u64,count:usize)->u32{
 std::panic::catch_unwind(||unsafe{if output.is_null()||count!=15{return 0}let Some(client)=CLIENT.get()else{return 0};let s=client.stats();let counters=[s.attempts,s.authenticated,s.disconnects,s.sent,s.received,s.expired,s.late,s.refused,s.max_latency_us,s.latency_p95_us,s.generation,s.retry_delay_ms,0,0,0];std::ptr::copy_nonoverlapping(counters.as_ptr(),output,count);1}).unwrap_or(0)
}
// Broker-internal C ABI. No Lua-facing export or second game-thread I/O owner.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nt2_local_ws_exchange(path:*const u16,path_length:usize,private:u32,input:*const u8,length:usize,output:*mut u8,capacity:usize,budget_ms:u32,wrote:*mut u32,app:*mut u8)->i32{
 let result=std::panic::catch_unwind(||unsafe{
  let began=std::time::Instant::now();
  if wrote.is_null()||app.is_null(){return false}*wrote=0;
  if path.is_null()||path_length==0||path_length>32768||input.is_null()||output.is_null()||length<32||length>crate::protocol::MAX_PAYLOAD||capacity<32||capacity>crate::protocol::MAX_PAYLOAD||budget_ms==0||budget_ms>12000||private>1{return false}
  let Ok(path)=String::from_utf16(std::slice::from_raw_parts(path,path_length))else{return false};
  // First module-path resolution and worker setup consume the caller's original
  // budget too. Transient setup errors do not become a lifetime reconnect limit.
  if CLIENT.get().is_none(){let Ok(path)=crate::private::module_descriptor(std::path::Path::new(&path))else{return false};let Ok(client)=crate::client::Client::new(path,private==1)else{return false};let _=CLIENT.set(client);}
  let Some(client)=CLIENT.get()else{return false};let budget=std::time::Duration::from_millis(u64::from(budget_ms)).saturating_sub(began.elapsed());if budget.is_zero(){return false}
  let (reply,sent)=client.exchange_bound(std::slice::from_raw_parts(input,length),budget);*wrote=u32::from(sent);
  let Ok((reply,peer_app))=reply else{return false};if reply.len()!=capacity{return false}std::ptr::copy_nonoverlapping(reply.as_ptr(),output,reply.len());std::ptr::copy_nonoverlapping(peer_app.as_ptr(),app,16);true
 });i32::from(result.unwrap_or(false))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nt2_local_ws_statistics(output:*mut u8,capacity:usize)->usize{
 std::panic::catch_unwind(||unsafe{if output.is_null()||capacity>4096{return 0}let Some(client)=CLIENT.get()else{return 0};let Ok(bytes)=serde_json::to_vec(&client.stats())else{return 0};if bytes.len()>capacity{return 0}std::ptr::copy_nonoverlapping(bytes.as_ptr(),output,bytes.len());bytes.len()}).unwrap_or(0)
}
