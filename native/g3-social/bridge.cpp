// UNAPPROVED source candidate. Own pipe prefix; stable g3-transport stays unchanged.
#ifndef NT2_PIPE_PREFIX
#define NT2_PIPE_PREFIX L"\\\\.\\pipe\\nt2-g3-social-dev-v1-"
#endif
#define nt2_g3_prepare nt2_g3_seedghost_prepare
#define nt2_g3_close nt2_g3_seedghost_close
#define nt2_g3_report nt2_g3_blocking_report
#include "../g3-transport/bridge.cpp"
#undef nt2_g3_prepare
#undef nt2_g3_close
#undef nt2_g3_report
static constexpr uint32_t SOCIAL_BYTES=32832;
static SRWLOCK social_lock=SRWLOCK_INIT;
static HANDLE social_worker=nullptr,social_stop=nullptr;
static uint8_t social_reply[SOCIAL_BYTES]={};
static uint32_t social_ack=0;
static ULONGLONG social_until=0;
static std::vector<uint8_t> social_report;
static uint64_t social_report_sequence=0,social_report_ack=0;
static bool social_report_pending=false;
static DWORD WINAPI social_run(void*){
 while(WaitForSingleObject(social_stop,16)==WAIT_TIMEOUT){
  std::vector<uint8_t> report;uint64_t sequence=0;
  AcquireSRWLockShared(&social_lock);if(social_report_pending){report=social_report;sequence=social_report_sequence;}ReleaseSRWLockShared(&social_lock);
  if(!report.empty()&&nt2_g3_blocking_report(gameplay_context,report.data(),static_cast<uint32_t>(report.size()),sequence)==1){AcquireSRWLockExclusive(&social_lock);if(social_report_sequence==sequence){social_report_ack=sequence;social_report_pending=false;}ReleaseSRWLockExclusive(&social_lock);}
  if(nt2_g3_status(gameplay_context)!=2){AcquireSRWLockExclusive(&social_lock);if(social_reply[5]!=2){memset(social_reply,0,SOCIAL_BYTES);social_until=0;}ReleaseSRWLockExclusive(&social_lock);}
  uint32_t length=0;AcquireSRWLockShared(&social_lock);memcpy(&length,social_reply+32,4);bool occupied=length!=0&&GetTickCount64()<social_until;uint32_t ack=social_ack;ReleaseSRWLockShared(&social_lock);
  if(occupied)continue;
  const std::wstring user=sid();if(user.empty())continue;const std::wstring path=NT2_PIPE_PREFIX+user;
  HANDLE pipe=CreateFileW(path.c_str(),GENERIC_READ|GENERIC_WRITE,0,nullptr,OPEN_EXISTING,FILE_FLAG_OVERLAPPED,nullptr);
  if(pipe==INVALID_HANDLE_VALUE)continue;if(!app(pipe)){CloseHandle(pipe);continue;}
  uint8_t request[64]={};memcpy(request,"NSP1\1",5);memcpy(request+8,gameplay_context,16);memcpy(request+24,&ack,4);
  std::vector<uint8_t> reply(SOCIAL_BYTES,0);ULONGLONG began=GetTickCount64();bool ok=transfer(pipe,true,request,64)&&transfer(pipe,false,reply.data(),SOCIAL_BYTES);CloseHandle(pipe);
  uint32_t generation=0,lease=0;memcpy(&generation,reply.data()+24,4);memcpy(&lease,reply.data()+28,4);memcpy(&length,reply.data()+32,4);
  if(ok&&memcmp(reply.data(),"NSR1\1",5)==0&&reply[5]==3&&memcmp(reply.data()+8,gameplay_context,16)==0&&generation!=0&&lease==0&&length==0){AcquireSRWLockExclusive(&social_lock);social_ack=0;memset(social_reply,0,SOCIAL_BYTES);social_until=0;ReleaseSRWLockExclusive(&social_lock);continue;}
  ok=ok&&(reply[5]==2||nt2_g3_status(gameplay_context)==2)&&memcmp(reply.data(),"NSR1\1",5)==0&&(reply[5]==1||reply[5]==2)&&reply[6]==0&&reply[7]==0&&memcmp(reply.data()+8,gameplay_context,16)==0&&generation!=0&&lease>0&&lease<=250&&length<=32768&&GetTickCount64()-began<lease;
  for(uint32_t i=36;i<40;i++)ok=ok&&reply[i]==0;for(uint32_t i=48;i<64;i++)ok=ok&&reply[i]==0;
  if(length)ok=ok&&length>=40&&memcmp(reply.data()+64,"NSF1\1",5)==0&&memcmp(reply.data()+72,gameplay_context,16)==0&&memcmp(reply.data()+88,&generation,4)==0;
  AcquireSRWLockExclusive(&social_lock);if(ok){memcpy(social_reply,reply.data(),SOCIAL_BYTES);social_until=began+lease;}else{memset(social_reply,0,SOCIAL_BYTES);social_until=0;}ReleaseSRWLockExclusive(&social_lock);
 }
 return 0;
}
static bool social_stop_worker(){if(!social_worker)return true;SetEvent(social_stop);if(WaitForSingleObject(social_worker,400)!=WAIT_OBJECT_0)return false;CloseHandle(social_worker);CloseHandle(social_stop);social_worker=nullptr;social_stop=nullptr;AcquireSRWLockExclusive(&social_lock);memset(social_reply,0,SOCIAL_BYTES);social_until=0;ReleaseSRWLockExclusive(&social_lock);return true;}
extern "C" __declspec(dllexport) int __cdecl nt2_g3_prepare(uint32_t* out,uint8_t* context){
 if(!social_stop_worker())return 0;int result=nt2_g3_seedghost_prepare(out,context);if(result!=1)return result;social_ack=0;social_report.clear();social_report_sequence=0;social_report_ack=0;social_report_pending=false;
 social_stop=CreateEventW(nullptr,TRUE,FALSE,nullptr);if(!social_stop){nt2_g3_seedghost_close(context);return 0;}social_worker=CreateThread(nullptr,0,social_run,nullptr,0,nullptr);if(!social_worker){CloseHandle(social_stop);social_stop=nullptr;nt2_g3_seedghost_close(context);return 0;}return 1;
}
extern "C" __declspec(dllexport) int __cdecl nt2_g3_close(const uint8_t* context){if(!current(context)||!social_stop_worker())return 0;return nt2_g3_seedghost_close(context);}
extern "C" __declspec(dllexport) int __cdecl nt2_g3_report(const uint8_t* context,const uint8_t* body,uint32_t size,uint64_t sequence){
 if(!current(context)||!body||size==0||size>32768||sequence==0)return 0;
 AcquireSRWLockExclusive(&social_lock);bool same=sequence==social_report_sequence&&size==social_report.size()&&memcmp(body,social_report.data(),size)==0;
 bool accepted=same&&sequence==social_report_ack;
 if(!social_report_pending&&sequence>social_report_ack){social_report.assign(body,body+size);social_report_sequence=sequence;social_report_pending=true;}
 ReleaseSRWLockExclusive(&social_lock);return accepted?1:0; // Success only after worker acknowledgement of these exact bytes.
}
extern "C" __declspec(dllexport) int __cdecl nt2_g3_social_lease(const uint8_t* context,uint8_t* out,uint32_t capacity){
 if(!current(context)||!out||capacity!=64||nt2_g3_status(context)!=2)return 0;AcquireSRWLockShared(&social_lock);ULONGLONG now=GetTickCount64();bool ok=social_reply[5]==1&&now<social_until;
 if(ok){memcpy(out,social_reply,64);uint32_t remaining=static_cast<uint32_t>(social_until-now);memcpy(out+28,&remaining,4);}ReleaseSRWLockShared(&social_lock);return ok?64:0;
}
extern "C" __declspec(dllexport) int __cdecl nt2_g3_social_next(const uint8_t* context,uint8_t* out,uint32_t capacity){
 if(!current(context)||!out||capacity!=32768||nt2_g3_status(context)!=2)return 0;AcquireSRWLockExclusive(&social_lock);uint32_t size=0;memcpy(&size,social_reply+32,4);
 if(GetTickCount64()>=social_until||size>32768)size=0;if(size){memcpy(out,social_reply+64,size);memcpy(&social_ack,social_reply+92,4);memset(social_reply+32,0,4);}ReleaseSRWLockExclusive(&social_lock);return static_cast<int>(size);
}
extern "C" __declspec(dllexport) int __cdecl nt2_g3_social_terminal(const uint8_t* context,uint8_t* out,uint32_t capacity){
 if(!current(context)||!out||capacity!=32768)return 0;AcquireSRWLockExclusive(&social_lock);uint32_t size=0;memcpy(&size,social_reply+32,4);
 if(social_reply[5]!=2||GetTickCount64()>=social_until||size!=41||social_reply[104]!=6)size=0;
 if(size){memcpy(out,social_reply+64,size);memcpy(&social_ack,social_reply+92,4);memset(social_reply+32,0,4);}ReleaseSRWLockExclusive(&social_lock);return static_cast<int>(size);
}
