// Separate uninstalled G3 candidate. Shared seed/inspection identity/consumption
// guards, with G3-only bounded bootstrap waits and verified heartbeat timeout handling.
// Game-thread movement exports only copy bounded frames; worker owns gameplay I/O.
#ifndef NT2_PIPE_PREFIX
#define NT2_PIPE_PREFIX L"\\\\.\\pipe\\nt2-g3-dev-v1-"
#endif
#define NT2_APP_HINT_SUFFIX L"\\NoitaTogether2\\g3-dev\\bridge-app.txt"
#define NT2_G3_VERIFIED_HEARTBEAT_TIMEOUT
#define NT2_G3_BOOTSTRAP_WAIT
#define nt2_g2_prepare nt2_g3_base_prepare
#define nt2_g2_seed_accepted nt2_g3_seed_accepted
#define nt2_g2_world_loaded nt2_g3_world_loaded
#define nt2_g2_status nt2_g3_status
#define nt2_g2_close nt2_g3_base_close
#include "../g2-bridge/bridge.cpp"
#include "../g3-bridge/frame.cpp"
static SRWLOCK gameplay_lock=SRWLOCK_INIT;
static HANDLE gameplay_worker=nullptr,gameplay_stop=nullptr;
static uint8_t outgoing[64]={},incoming[864]={},gameplay_context[16]={};
static bool queued=false;static uint32_t local_sequence=0;
static ULONGLONG incoming_until=0;
// Fixed last heartbeat result only, no identifiers/paths/Windows error strings.
// 0 acknowledged;2 identity;3 verified timeout;4 I/O;5 malformed;6 busy;
// 7 unavailable;8 SID;9 negative reply. Never changes status or its original age.
extern "C" __declspec(dllexport) int __cdecl nt2_g3_heartbeat_diagnostic(const uint8_t* context){
 if(!current(context)||!liveness)return 7;
 return (int)InterlockedCompareExchange(&liveness->failure,0,0);
}
static bool gameplay_exchange(uint8_t* request,uint8_t* reply,bool& denied,ULONGLONG& deadline){
 denied=false;deadline=0;
 const std::wstring user=sid();if(user.empty())return false;
 const std::wstring path=NT2_PIPE_PREFIX+user;
 HANDLE pipe=CreateFileW(path.c_str(),GENERIC_READ|GENERIC_WRITE,0,nullptr,OPEN_EXISTING,FILE_FLAG_OVERLAPPED,nullptr);
 if(pipe==INVALID_HANDLE_VALUE&&GetLastError()==ERROR_PIPE_BUSY&&WaitNamedPipeW(path.c_str(),80))pipe=CreateFileW(path.c_str(),GENERIC_READ|GENERIC_WRITE,0,nullptr,OPEN_EXISTING,FILE_FLAG_OVERLAPPED,nullptr);
 if(pipe==INVALID_HANDLE_VALUE)return false;if(!app(pipe)){CloseHandle(pipe);return false;}
 ULONGLONG began=GetTickCount64();bool ok=transfer(pipe,true,request,64)&&transfer(pipe,false,reply,864);CloseHandle(pipe);
 // A complete negative reply from the verified owner is a revocation, not I/O loss.
 if(ok&&memcmp(reply,"NG3R",4)==0&&reply[4]==2&&reply[5]==0){
  bool empty=true;for(uint32_t i=6;i<864;i++)empty=empty&&reply[i]==0;
  if(empty){denied=true;return false;}
 }
 uint32_t lease=0;memcpy(&lease,reply+24,4);
 ok=ok&&memcmp(reply,"NG3R",4)==0&&reply[4]==2&&reply[5]==1&&reply[6]<=4&&reply[7]==0&&memcmp(reply+8,gameplay_context,16)==0&&lease>0&&lease<=200&&GetTickCount64()-began<lease;
 ok=ok&&reply[28]<=32;for(uint32_t i=29;i<32;i++)ok=ok&&reply[i]==0;
 if(ok){deadline=began+lease;ULONGLONG now=GetTickCount64();if(now>=deadline)return false;uint32_t remaining=static_cast<uint32_t>(deadline-now);memcpy(reply+24,&remaining,4);}return ok;
}
static DWORD WINAPI gameplay(void*){
 while(WaitForSingleObject(gameplay_stop,20)==WAIT_TIMEOUT){
  uint8_t request[64]={},reply[864]={};
  if(nt2_g3_status(gameplay_context)!=2){AcquireSRWLockExclusive(&gameplay_lock);memset(incoming,0,864);queued=false;ReleaseSRWLockExclusive(&gameplay_lock);continue;}
  AcquireSRWLockExclusive(&gameplay_lock);
  if(queued){memcpy(request,outgoing,64);queued=false;}else{memcpy(request,"NTG3",4);request[4]=1;request[6]=2;memcpy(request+8,gameplay_context,16);}
  ReleaseSRWLockExclusive(&gameplay_lock);
  bool denied=false;ULONGLONG deadline=0;bool ok=gameplay_exchange(request,reply,denied,deadline);
  AcquireSRWLockExclusive(&gameplay_lock);if(ok){memcpy(incoming,reply,864);incoming_until=deadline;}
  else if(denied||GetTickCount64()>=incoming_until){memset(incoming,0,864);incoming_until=0;}ReleaseSRWLockExclusive(&gameplay_lock);
 }
 return 0;
}
static bool stop_gameplay(){
 if(!gameplay_worker)return true;SetEvent(gameplay_stop);
 if(WaitForSingleObject(gameplay_worker,400)!=WAIT_OBJECT_0)return false;
 CloseHandle(gameplay_worker);CloseHandle(gameplay_stop);gameplay_worker=nullptr;gameplay_stop=nullptr;
 AcquireSRWLockExclusive(&gameplay_lock);queued=false;memset(incoming,0,864);ReleaseSRWLockExclusive(&gameplay_lock);return true;
}
extern "C" __declspec(dllexport) int __cdecl nt2_g3_prepare(uint32_t* out,uint8_t* context){
 if(!stop_gameplay())return 0;int result=nt2_g3_base_prepare(out,context);if(result!=1)return result;
 memcpy(gameplay_context,context,16);local_sequence=0;queued=false;memset(incoming,0,864);
 gameplay_stop=CreateEventW(nullptr,TRUE,FALSE,nullptr);if(!gameplay_stop){nt2_g3_base_close(context);return 0;}
 gameplay_worker=CreateThread(nullptr,0,gameplay,nullptr,0,nullptr);
 if(!gameplay_worker){CloseHandle(gameplay_stop);gameplay_stop=nullptr;nt2_g3_base_close(context);return 0;}return 1;
}
extern "C" __declspec(dllexport) int __cdecl nt2_g3_close(const uint8_t* context){
 if(!current(context))return 0;if(!stop_gameplay())return 0;return nt2_g3_base_close(context);
}
extern "C" __declspec(dllexport) int __cdecl nt2_g3_move(const uint8_t* context,uint32_t sequence,int32_t x,int32_t y,int32_t vx,int32_t vy,uint32_t facing){
 if(!current(context)||nt2_g3_status(context)!=2||sequence<=local_sequence)return 0;
 uint8_t frame[64];if(NT2G3EncodePose(frame,64,context,sequence,x,y,vx,vy,facing)!=64)return 0;
 AcquireSRWLockExclusive(&gameplay_lock);memcpy(outgoing,frame,64);queued=true;local_sequence=sequence;ReleaseSRWLockExclusive(&gameplay_lock);return 1;
}
extern "C" __declspec(dllexport) int __cdecl nt2_g3_peers(const uint8_t* context,uint8_t* out,uint32_t capacity){
 if(!current(context)||!out||capacity!=864||nt2_g3_status(context)!=2)return 0;
 AcquireSRWLockShared(&gameplay_lock);bool valid=GetTickCount64()<incoming_until;memcpy(out,incoming,864);ReleaseSRWLockShared(&gameplay_lock);
 return valid&&memcmp(out,"NG3R",4)==0&&out[5]==1?864:0;
}
extern "C" __declspec(dllexport) int __cdecl nt2_g3_report(const uint8_t* context,const uint8_t* body,uint32_t size,uint64_t sequence){
 if(!current(context)||!body||size==0||size>32768||sequence==0)return 0;
 const std::wstring user=sid();if(user.empty())return 0;const std::wstring path=NT2_PIPE_PREFIX+user;
 HANDLE pipe=CreateFileW(path.c_str(),GENERIC_READ|GENERIC_WRITE,0,nullptr,OPEN_EXISTING,FILE_FLAG_OVERLAPPED,nullptr);
 if(pipe==INVALID_HANDLE_VALUE&&GetLastError()==ERROR_PIPE_BUSY&&WaitNamedPipeW(path.c_str(),80))pipe=CreateFileW(path.c_str(),GENERIC_READ|GENERIC_WRITE,0,nullptr,OPEN_EXISTING,FILE_FLAG_OVERLAPPED,nullptr);
 if(pipe==INVALID_HANDLE_VALUE)return 0;if(!app(pipe)){CloseHandle(pipe);return 0;}
 std::vector<uint8_t> frame(48+size,0);const uint32_t fields[4]={0x3247544e,3,7,size};
 memcpy(frame.data(),fields,16);memcpy(frame.data()+16,&sequence,8);memcpy(frame.data()+24,context,16);memcpy(frame.data()+48,body,size);
 uint8_t reply[32]={};bool ok=transfer(pipe,true,frame.data(),static_cast<DWORD>(frame.size()))&&transfer(pipe,false,reply,32);CloseHandle(pipe);
 uint32_t values[4];memcpy(values,reply,16);bool accepted=ok&&values[0]==0x3247544e&&values[1]==3&&values[2]==1&&(values[3]==1||values[3]==2)&&memcmp(reply+16,context,16)==0;
 if(accepted&&liveness){InterlockedExchange(&liveness->status,static_cast<LONG>(values[3]));InterlockedExchange64(&liveness->updated,static_cast<LONG64>(GetTickCount64()));}return accepted?1:0;
}
