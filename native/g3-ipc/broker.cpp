// Shared native broker. One authenticated persistent WebSocket; unchanged bounded lanes.
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <sddl.h>
#include <bcrypt.h>
#include <string>
#include <vector>
#include <deque>
#include <memory>
#include <cstring>
#include "abi.h"
#pragma comment(lib,"advapi32.lib")
#pragma comment(lib,"bcrypt.lib")
#ifndef NT2_PIPE_PREFIX
#define NT2_PIPE_PREFIX L"\\\\.\\pipe\\nt2-g3-c2-dev-v1-"
#endif
#define NT2_APP_HINT_SUFFIX L"\\NoitaTogether2\\g3-c2-dev\\bridge-app.txt"
static constexpr uint32_t SOCIAL_BYTES=41040,MAX_BYTES=131072;
static INIT_ONCE initialized=INIT_ONCE_STATIC_INIT;static SRWLOCK lock=SRWLOCK_INIT;static HANDLE wake=nullptr;
struct Job{uint32_t op,payload;uint64_t generation,sequence;ULONGLONG until;uint8_t context[16];std::vector<uint8_t> body,debug_body,bank_body;uint64_t debug_sequence=0,bank_sequence=0;bool c1_completion=false;std::shared_ptr<const std::vector<uint8_t>> original;};
static std::deque<Job> controls;static Job report_job{};
static bool report_pending=false,report_inflight=false,report_uncertain=false,pose_pending=false;static ULONGLONG report_retry_at=0;
static uint64_t generation=0,report_sequence=0,report_ack=0,actual_seed=0;static bool seed_bound=false;
static uint8_t transport_app[16]={};static uint8_t resume_association[16]={};static bool resuming=false;
static uint8_t association_record[1024]={};static uint32_t association_size=0,saved_lookup=4,resume_decision=3;static std::vector<uint8_t> saved_record;static ULONGLONG metadata_at=0,lookup_at=0,resume_retry_at=0;
static uint8_t context[16]={},pose[64]={},peers[864]={},social[SOCIAL_BYTES]={};
static bool debug_enabled=true;static std::vector<uint8_t> debug_input;static uint64_t debug_sequence=0;
static std::vector<uint8_t> bank_input,c1_input;static uint64_t bank_sequence=0,c1_sequence=0;
static std::vector<uint8_t> social_input;static uint64_t social_input_sequence=0;
static std::shared_ptr<const std::vector<uint8_t>> report_original=std::make_shared<const std::vector<uint8_t>>();
static uint32_t control_pending=0,control_finished=0;static bool continue_refused=false;static uint32_t authority_generation=0;static uint32_t origins=0,seed=0,world_policy=0,phase=0,status=0,failure=7,pose_sequence=0,social_ack=0;
// Read-only display cache. Never contributes to status/health/play decisions.
static uint32_t display_code=0,display_epoch=0;static uint64_t display_generation=0;static ULONGLONG display_until=0;
static uint8_t lobby_frame[256]={};static ULONGLONG lobby_until=0,lobby_at=0;static bool lobby_requested=false;
static ULONGLONG heartbeat_at=0,health_until=0,peers_until=0,social_until=0;
// A failed read is availability; an actual denied/malformed reply is not.
// Keep a denial until a fresh validated batch or a new native scope replaces it.
static bool peers_rejected=false;
static DWORD worker_id=0;static uint32_t completed[4]={},coalesced=0,refused=0,max_bytes=0;
static constexpr uint32_t STARTUP_BYTES=16384;
static uint8_t startup_record[STARTUP_BYTES]={},startup_bootstrap[16]={};static bool startup_offered=false,startup_latched=false;static ULONGLONG startup_deadline=0,startup_next=0,startup_began=0;static uint32_t startup_reason=0,startup_original=2,startup_queue_ms=0,startup_claim_ms=0,startup_reply_ms=0,startup_attempts=0;
static int startup(uint32_t,uint8_t*,uint32_t,uint8_t*,uint32_t);
// Metadata frame5 contains no gameplay grants, packets or effect permits.
static bool race_metadata_frame(const uint8_t* frame){uint32_t n=0;memcpy(&n,frame+32840,4);return frame[5]==5&&n>=8&&n<=8192&&memcmp(frame+32848,"{\"race\":",8)==0;}
static bool active(uint32_t origin,const uint8_t* c){return origin>=1&&origin<=2&&c&&phase!=0&&phase!=9&&(origins&(1u<<origin))&&memcmp(c,context,16)==0;}
static void install_peers(const std::vector<uint8_t>& reply,bool ok,ULONGLONG began,ULONGLONG now,uint32_t reason=0){
 if(!ok&&reason==2){memset(peers,0,864);peers_until=0;peers_rejected=true;return;}
 uint32_t lease=0;if(reply.size()>=28)memcpy(&lease,reply.data()+24,4);
 bool shape=ok&&reply.size()==864&&memcmp(reply.data(),"NG3R\2\1",6)==0&&reply[6]<=4&&reply[7]==0&&memcmp(reply.data()+8,context,16)==0&&reply[28]<=32&&lease>0&&lease<=200;
 if(shape){for(uint32_t i=29;i<32;i++)shape=shape&&reply[i]==0;}
 if(shape&&now>=began&&now-began<lease){memcpy(peers,reply.data(),864);peers_until=began+lease;peers_rejected=false;}
 else if(ok||now>=peers_until){memset(peers,0,864);peers_until=0;if(ok&&!shape)peers_rejected=true;}
}
static uint32_t bytes(){return static_cast<uint32_t>(report_original->size()+social_input.size()+debug_input.size()+bank_input.size()+c1_input.size())+SOCIAL_BYTES+SOCIAL_BYTES+1024+static_cast<uint32_t>(controls.size())*128;}
extern "C" int nt2_local_ws_exchange(const uint16_t*,size_t,uint32_t,const uint8_t*,size_t,uint8_t*,size_t,uint32_t,uint32_t*,uint8_t*);
extern "C" size_t nt2_local_ws_statistics(uint8_t*,size_t);
extern "C" uint32_t nt2_local_ws_counters(uint64_t*,size_t);
static std::wstring bootstrap_path(){
 HMODULE module=nullptr;wchar_t path[32768];
 if(!GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS|GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,reinterpret_cast<LPCWSTR>(&bootstrap_path),&module))return L"";
 DWORD n=GetModuleFileNameW(module,path,32768);if(!n||n>=32768)return L"";
 std::wstring root(path,n);auto end=root.find_last_of(L"\\/");if(end==std::wstring::npos)return L"";root.resize(end);end=root.find_last_of(L"\\/");if(end==std::wstring::npos)return L"";root.resize(end);return root+L"\\.nt2-local\\bridge.json";
}
static bool call(const std::wstring& descriptor,std::vector<uint8_t>& request,std::vector<uint8_t>& reply,ULONGLONG until,uint32_t& reason,bool& wrote){
 reason=7;wrote=false;ULONGLONG now=GetTickCount64();if(descriptor.empty()||now>=until)return false;
 uint32_t sent=0;uint8_t peer_app[16]={};
#ifdef NT2_IPC_PRIVATE
 const uint32_t private_endpoint=1;
#else
 const uint32_t private_endpoint=0;
#endif
 int result=nt2_local_ws_exchange(reinterpret_cast<const uint16_t*>(descriptor.data()),descriptor.size(),private_endpoint,request.data(),request.size(),reply.data(),reply.size(),static_cast<uint32_t>(until-now),&sent,peer_app);
 // A reply is bound to the authenticated app incarnation which produced it.
 // A restarted app supplies a new local lease epoch; retain world/context and
 // originals, withdraw cached play, then learn its epoch through held collection.
 if(result==1){AcquireSRWLockExclusive(&lock);uint8_t any=0;for(uint32_t i=0;i<16;i++)any|=transport_app[i];if(any&&memcmp(transport_app,peer_app,16)!=0){status=0;health_until=0;authority_generation=0;peers_until=0;social_until=0;memset(peers,0,sizeof(peers));memset(social,0,sizeof(social));}memcpy(transport_app,peer_app,16);ReleaseSRWLockExclusive(&lock);}
 wrote=sent!=0;reason=result?0:(wrote?3:7);return result==1&&GetTickCount64()<until;
}
// Lifecycle/inspection originals reserve one250ms total queue+I/O budget.
// Timely40ms active optional work plus inspection must fit, without restarting
// any deadline at dispatch. Heartbeat/movement/social retain80ms total budgets.
static Job job(uint32_t op,uint32_t payload=0){Job j{};j.op=op;j.payload=payload;j.generation=generation;j.until=(op==16||op==18||op==19)?startup_deadline:GetTickCount64()+((op==1||op==3||op==4||op==7||op==10||op==11)?250:80);memcpy(j.context,context,16);return j;}
static bool enqueue(uint32_t op,uint32_t payload=0){if(control_pending&(1u<<op))return true;if(control_finished&(1u<<op))return false;if(controls.size()>=8){refused++;return false;}control_pending|=1u<<op;controls.push_back(job(op,payload));SetEvent(wake);return true;}
static DWORD WINAPI run(void*){
 AcquireSRWLockExclusive(&lock);worker_id=GetCurrentThreadId();ReleaseSRWLockExclusive(&lock);const std::wstring path=bootstrap_path();uint32_t burst=0;bool social_turn=false;
 for(;;){
  Job j{};bool selected=false;uint32_t lane=0;ULONGLONG now=GetTickCount64();
  AcquireSRWLockExclusive(&lock);
  // Reserve control time before the next heartbeat; optional jobs cannot run
  // into its original deadline. No export waits on this worker.
  if(phase>=2&&phase<9&&now+80>=heartbeat_at){j=job(5);selected=true;heartbeat_at=now+2000;lane=0;}
  else if(phase==1&&!resuming&&GetTickCount64()<startup_deadline&&now>=startup_next&&!(control_pending&((1u<<16)|(1u<<19)))){enqueue(startup_offered?16:19);startup_next=now+15;}
  else if(phase==1&&resuming&&now>=resume_retry_at&&!(control_pending&(1u<<12))){control_finished&=~(1u<<12);enqueue(12);controls.back().body.assign(resume_association,resume_association+16);const uint8_t* value=reinterpret_cast<const uint8_t*>(&actual_seed);controls.back().body.insert(controls.back().body.end(),value,value+8);resume_retry_at=now+100;}
  else if(phase>=6&&phase<9&&now>=metadata_at&&!(control_pending&(1u<<13))){control_finished&=~(1u<<13);enqueue(13);metadata_at=now+500;}
  else if(!controls.empty()&&(burst<8||!report_pending)){j=std::move(controls.front());controls.pop_front();selected=true;burst++;lane=0;}
  else if(report_pending&&!report_inflight){j=std::move(report_job);report_inflight=true;selected=true;burst=0;lane=1;}
  else if(lobby_requested&&now>=lobby_at){j=job(20);selected=true;lane=3;lobby_at=now+500;}
  else if(phase>=2&&phase<9&&now<health_until){
   if((social_turn||status!=2)&&(origins&4)&&(!social_input.empty()||!(social[5]!=0&&now<social_until&&*reinterpret_cast<uint32_t*>(social+32)!=0))){j=job(9,social_ack);j.body=social_input;j.sequence=social_input.empty()?0:social_input_sequence;j.debug_body=debug_input;j.debug_sequence=debug_input.empty()?0:debug_sequence;j.c1_completion=!c1_input.empty();j.bank_body=j.c1_completion?c1_input:bank_input;j.bank_sequence=j.c1_completion?c1_sequence:(bank_input.empty()?0:bank_sequence);lane=3;selected=true;}
   else if(status==2){j=job(8);j.body.assign(64,0);if(pose_pending){memcpy(j.body.data(),pose,64);pose_pending=false;}else{memcpy(j.body.data(),"NTG3\1",5);j.body[6]=2;memcpy(j.body.data()+8,context,16);}lane=2;selected=true;}
   social_turn=!social_turn;burst=0;
  }
  max_bytes=(bytes()>max_bytes)?bytes():max_bytes;ReleaseSRWLockExclusive(&lock);
  if(!selected){WaitForSingleObject(wake,5);continue;}
  bool continuing=false; // Mode is never inferred from files or failure.
  std::vector<uint8_t> request,reply;ULONGLONG began=GetTickCount64();
  if(j.op==16||j.op==18||j.op==19||j.op==17){request.assign(j.op==17?80:32,0);uint32_t fields[4]={0x3247544e,9,j.op,0};memcpy(request.data(),fields,16);memcpy(request.data()+16,j.context,16);if(j.op==17&&j.body.size()==48)memcpy(request.data()+32,j.body.data(),48);reply.assign(STARTUP_BYTES,0);}
  else if(j.op==8){request=j.body;reply.assign(864,0);}
  else if(j.op==9){request.assign(64+j.body.size()+j.debug_body.size()+j.bank_body.size(),0);memcpy(request.data(),"NSP1\1",5);uint32_t debug_size=static_cast<uint32_t>(j.debug_body.size());memcpy(request.data()+28,&debug_size,4);memcpy(request.data()+32,&j.debug_sequence,8);if(debug_size)memcpy(request.data()+64+j.body.size(),j.debug_body.data(),debug_size);uint32_t bank_size=static_cast<uint32_t>(j.bank_body.size());memcpy(request.data()+40,&j.bank_sequence,8);memcpy(request.data()+60,&bank_size,4);if(bank_size)memcpy(request.data()+64+j.body.size()+j.debug_body.size(),j.bank_body.data(),bank_size);memcpy(request.data()+8,j.context,16);memcpy(request.data()+24,&j.payload,4);uint32_t length=static_cast<uint32_t>(j.body.size());memcpy(request.data()+48,&j.sequence,8);memcpy(request.data()+56,&length,4);if(length)memcpy(request.data()+64,j.body.data(),length);reply.assign(SOCIAL_BYTES,0);}
  else if(j.op==20){request.assign(32,0);uint32_t fields[4]={0x3247544e,10,20,0};memcpy(request.data(),fields,16);memcpy(request.data()+16,j.context,16);reply.assign(256,0);}
  else if(j.op==13){request.assign(32,0);uint32_t fields[4]={0x3247544e,8,13,0};memcpy(request.data(),fields,16);memcpy(request.data()+16,j.context,16);reply.assign(1072,0);}
  else if(j.op==14){request.assign(32+j.body.size(),0);uint32_t fields[4]={0x3247544e,8,14,static_cast<uint32_t>(j.body.size())};memcpy(request.data(),fields,16);memcpy(request.data()+16,j.context,16);memcpy(request.data()+32,j.body.data(),j.body.size());reply.assign(48,0);}
  else if(j.op==15){request.assign(32,0);uint32_t fields[4]={0x3247544e,8,15,0};memcpy(request.data(),fields,16);memcpy(request.data()+16,j.context,16);reply.assign(48,0);}
  else if(j.op==12){request.assign(56,0);uint32_t fields[4]={0x3247544e,7,12,0};memcpy(request.data(),fields,16);memcpy(request.data()+16,j.context,16);memcpy(request.data()+32,j.body.data(),24);reply.assign(48,0);}
  else if(j.op==11){request.assign(40,0);uint32_t fields[4]={0x3247544e,6,11,j.payload};memcpy(request.data(),fields,16);memcpy(request.data()+16,j.context,16);memcpy(request.data()+32,j.body.data()+4,4);reply.assign(48,0);}
  else if(j.op==7){request.assign(48+j.original->size(),0);uint32_t fields[4]={0x3247544e,5,7,static_cast<uint32_t>(j.original->size())};memcpy(request.data(),fields,16);memcpy(request.data()+16,&j.sequence,8);memcpy(request.data()+24,j.context,16);memcpy(request.data()+48,j.original->data(),j.original->size());reply.assign(48,0);}
  else{request.assign(32,0);uint32_t fields[4]={0x3247544e,4,j.op,j.payload};memcpy(request.data(),fields,16);memcpy(request.data()+16,j.context,16);reply.assign(48,0);}
  bool wrote=false;uint32_t reason=0;bool ok=call(path,request,reply,j.until,reason,wrote);
  AcquireSRWLockExclusive(&lock);
  if(j.generation!=generation||memcmp(j.context,context,16)){ReleaseSRWLockExclusive(&lock);continue;}
  completed[lane]++;
  if(j.op==20){
   lobby_until=0;memset(lobby_frame,0,sizeof(lobby_frame));uint32_t code=0,lease=0,length=0;
   memcpy(&code,reply.data()+8,4);memcpy(&lease,reply.data()+12,4);memcpy(&length,reply.data()+32,4);
   bool valid=ok&&reply.size()==256&&memcmp(reply.data(),"NLD1\1\0\0\0",8)==0&&memcmp(reply.data()+16,j.context,16)==0&&code>=1&&code<=5&&lease>0&&lease<=2000&&length>0&&length<=192&&GetTickCount64()-began<lease;
   for(uint32_t i=36;i<64;i++)valid=valid&&reply[i]==0;
   for(uint32_t i=0;i<length&&i<192;i++)valid=valid&&reply[64+i]>=32&&reply[64+i]!=127;
   if(valid){memcpy(lobby_frame,reply.data(),256);lobby_until=began+lease;}
   ReleaseSRWLockExclusive(&lock);continue;
  }
  // A queued original finishes once, including pre-write expiry or uncertain
  // writes. Polls use its cached outcome; they never create an automatic replay.
  if(lane==0&&j.op!=5&&j.op<16){uint32_t bit=1u<<(continuing?1:j.op);control_pending&=~bit;control_finished|=bit;}
  if(j.op>=16&&j.op<=19){
   control_pending&=~(1u<<j.op);startup_attempts++;uint32_t fields[12]={};if(reply.size()>=192)memcpy(fields,reply.data(),48);
   uint32_t source_kind=2;if(reply.size()>=192)memcpy(&source_kind,reply.data()+176,4);
   bool valid=ok&&reply.size()==STARTUP_BYTES&&fields[0]==0x3247544e&&fields[1]==9&&fields[2]==16&&source_kind<=1;
   if(j.op==19){valid=valid&&memcmp(reply.data()+160,j.context,16)==0&&fields[3]==2&&fields[4]==0;if(valid){memcpy(context,reply.data()+48,16);memcpy(startup_record,reply.data(),STARTUP_BYTES);startup_offered=true;}else if(ok){startup_reason=fields[4]?fields[4]:8;startup_original=0;phase=9;failure=startup_reason;}}
   else if(j.op==17){/* scoped retirement preserves the original diagnostic */}
   else{uint32_t offered_kind=2;memcpy(&offered_kind,startup_record+176,4);valid=valid&&source_kind==offered_kind&&memcmp(reply.data()+48,context,16)==0&&memcmp(reply.data()+64,startup_record+64,80)==0;
    if(valid&&fields[3]==1&&fields[4]==0&&fields[8]>0&&fields[8]<=12000&&fields[7]!=0&&(fields[6]==1||fields[6]==2||fields[6]==5)&&fields[9]>0&&fields[9]<=STARTUP_BYTES-192&&(source_kind==1||memcmp(reply.data()+64,context,16)==0)){
     memcpy(startup_record,reply.data(),STARTUP_BYTES);seed=fields[5];world_policy=fields[6];authority_generation=fields[7];health_until=began+fields[8];heartbeat_at=GetTickCount64()+2000;startup_queue_ms=fields[10];startup_claim_ms=fields[11];startup_reply_ms=static_cast<uint32_t>(GetTickCount64()-began);startup_reason=0;startup_original=1;phase=2;status=1;
    }else if(valid&&fields[3]==2&&fields[4]==0){memcpy(startup_record,reply.data(),STARTUP_BYTES);startup_next=GetTickCount64()+15;}
    else if(ok){startup_reason=fields[4]?fields[4]:8;startup_original=0;phase=9;status=0;failure=startup_reason;}
   }
   if(!ok){startup_reason=reason==6?11:reason==2?12:wrote?13:10;startup_next=GetTickCount64()+25;if(GetTickCount64()>=startup_deadline){phase=9;startup_original=0;}}
  }
  else if(j.op==8){install_peers(reply,ok,began,GetTickCount64(),reason);}
  else if(j.op==9){uint32_t epoch=0,lease=0,length=0;memcpy(&epoch,reply.data()+24,4);memcpy(&lease,reply.data()+28,4);memcpy(&length,reply.data()+32,4);
   bool optional_ok=ok&&memcmp(reply.data(),"NSR1\1",5)==0&&reply[5]<=3&&reply[6]<=1&&reply[7]==0&&memcmp(reply.data()+8,context,16)==0&&epoch!=0&&length<=32768&&lease<=250;
   for(uint32_t i=36;i<40;i++)optional_ok=optional_ok&&reply[i]==0;
   optional_ok=optional_ok&&((reply[5]==0||reply[5]==3)?lease==0&&length==0:lease>0);
   if(optional_ok){debug_enabled=reply[6]!=0;uint64_t ack=0;memcpy(&ack,reply.data()+56,8);if(j.debug_sequence!=0&&ack==j.debug_sequence&&debug_sequence==j.debug_sequence)debug_input.clear();}
   if(ok&&memcmp(reply.data(),"NSR1\1",5)==0&&reply[5]==3&&reply[6]<=1&&reply[7]==0&&memcmp(reply.data()+8,context,16)==0&&epoch!=0&&lease==0&&length==0){social_ack=0;memset(social,0,SOCIAL_BYTES);social_until=0;ReleaseSRWLockExclusive(&lock);continue;}
   // C1 policy bootstrap is inert protected-world metadata. It never grants
   // a social lease, publishes input, carries a frame, or acknowledges work.
   if(reply[5]==4||reply[5]==5){
    uint32_t metadata_size=0;uint64_t bank_ack=0,input_ack=0;memcpy(&metadata_size,reply.data()+32840,4);memcpy(&bank_ack,reply.data()+32832,8);memcpy(&input_ack,reply.data()+48,8);
    bool metadata=ok&&memcmp(reply.data(),"NSR1\1",5)==0&&reply[6]<=1&&reply[7]==0&&memcmp(reply.data()+8,context,16)==0&&epoch!=0&&epoch==authority_generation&&(status==2||(status==1&&race_metadata_frame(reply.data())))&&now<health_until&&lease>0&&lease<=75&&length==0&&metadata_size>0&&metadata_size<=8192&&(reply[5]==4?bank_ack==0:bank_ack<=c1_sequence)&&input_ack==0&&GetTickCount64()-began<lease;
    for(uint32_t i=36;i<64;i++)metadata=metadata&&reply[i]==0;
    for(uint32_t i=32844;i<32848;i++)metadata=metadata&&reply[i]==0;
    if(metadata){if(reply[5]==5&&j.c1_completion&&j.bank_sequence!=0&&bank_ack==j.bank_sequence&&c1_sequence==j.bank_sequence)c1_input.clear();memcpy(social,reply.data(),SOCIAL_BYTES);social_until=began+lease;}else{memset(social,0,SOCIAL_BYTES);social_until=0;}
    ReleaseSRWLockExclusive(&lock);continue;
   }
   bool valid=ok&&memcmp(reply.data(),"NSR1\1",5)==0&&(reply[5]==1||reply[5]==2)&&reply[6]<=1&&reply[7]==0&&memcmp(reply.data()+8,context,16)==0&&epoch!=0&&lease>0&&lease<=250&&length<=32768&&GetTickCount64()-began<lease;
   for(uint32_t i=36;i<40;i++)valid=valid&&reply[i]==0;uint64_t observed_input_ack=0;memcpy(&observed_input_ack,reply.data()+48,8);valid=valid&&observed_input_ack<=social_input_sequence;
   if(length)valid=valid&&length>=40&&memcmp(reply.data()+64,"NSF1\1",5)==0&&memcmp(reply.data()+72,context,16)==0&&memcmp(reply.data()+88,&epoch,4)==0;
   uint32_t bank_length=0;uint64_t bank_ack=0;memcpy(&bank_ack,reply.data()+32832,8);memcpy(&bank_length,reply.data()+32840,4);valid=valid&&bank_length<=8192&&bank_ack<=(j.c1_completion?c1_sequence:bank_sequence);
   if(valid){if(j.bank_sequence!=0&&bank_ack==j.bank_sequence){if(j.c1_completion&&c1_sequence==j.bank_sequence)c1_input.clear();else if(!j.c1_completion&&bank_sequence==j.bank_sequence)bank_input.clear();}uint64_t input_ack=0;memcpy(&input_ack,reply.data()+48,8);if(j.sequence!=0&&input_ack==j.sequence&&social_input_sequence==j.sequence)social_input.clear();memcpy(social,reply.data(),SOCIAL_BYTES);social_until=began+lease;}else{memset(social,0,SOCIAL_BYTES);social_until=0;}}
  else if(j.op==13||j.op==14||j.op==15){uint32_t fields[4]={};memcpy(fields,reply.data(),16);bool valid=ok&&fields[0]==0x3247544e&&fields[1]==8&&fields[2]==1&&memcmp(reply.data()+16,context,16)==0;
   if(j.op==13){
    display_code=0;display_epoch=0;display_generation=0;display_until=0;
    if(valid&&fields[3]>0&&fields[3]<=1024){association_size=fields[3];memcpy(association_record,reply.data()+48,association_size);
     uint32_t d[4]={};memcpy(d,reply.data()+32,16);if(d[0]==1&&d[1]>=1&&d[1]<=4&&d[2]>0&&d[2]<=12000&&d[3]!=0&&d[3]==authority_generation&&GetTickCount64()-began<d[2]){display_code=d[1];display_epoch=d[3];display_generation=j.generation;display_until=began+d[2];}
    }else{association_size=0;memset(association_record,0,sizeof(association_record));}
   }
   if(j.op==14&&valid&&fields[3]<=6)saved_lookup=fields[3];
   else if(j.op==14&&ok&&fields[0]==0x3247544e&&fields[1]==8&&fields[2]==0&&fields[3]==0&&memcmp(reply.data()+16,context,16)==0)saved_lookup=0;
  }
  else{uint32_t fields[4]={},lease=0,epoch=0;uint64_t sequence=0;memcpy(fields,reply.data(),16);memcpy(&lease,reply.data()+32,4);memcpy(&epoch,reply.data()+36,4);memcpy(&sequence,reply.data()+40,8);
   bool valid=ok&&fields[0]==0x3247544e&&fields[1]==(j.op==7?5u:j.op==11?6u:j.op==12?7u:4u)&&fields[2]==1&&memcmp(reply.data()+16,context,16)==0&&((j.op==6&&lease==0)||(lease>0&&lease<=12000&&epoch!=0&&(authority_generation==0||epoch==authority_generation)&&GetTickCount64()-began<lease))&&sequence==(j.op==7?j.sequence:0);
   if(valid&&epoch)authority_generation=epoch;
   if(ok&&!valid)reason=fields[2]==0?9:5;
   if(j.op==12){if(ok&&fields[0]==0x3247544e&&fields[1]==7&&memcmp(reply.data()+16,context,16)==0&&fields[2]==0&&fields[3]==3){resume_decision=3;resume_retry_at=GetTickCount64()+100;ReleaseSRWLockExclusive(&lock);continue;}if(valid&&(fields[3]==1||fields[3]==2||fields[3]==5)&&(fields[3]==2||actual_seed<=UINT32_MAX)){resume_decision=1;world_policy=fields[3];seed=world_policy==2?0:static_cast<uint32_t>(actual_seed);seed_bound=true;phase=4;status=1;health_until=began+lease;heartbeat_at=GetTickCount64()+2000;}else if(ok&&fields[0]==0x3247544e&&fields[1]==7&&fields[2]==0&&memcmp(reply.data()+16,context,16)==0&&(fields[3]==4||fields[3]==6)){resume_decision=fields[3]==6?2:4;phase=9;status=0;failure=9;}else{resume_decision=3;phase=1;status=0;resume_retry_at=GetTickCount64()+250;}}
   if(j.op==1){if(valid){seed=fields[3];status=1;health_until=began+lease;heartbeat_at=GetTickCount64()+2000;enqueue(10);controls.back().until=j.until;}else{phase=9;status=0;}}
   if(j.op==10){if(valid&&(fields[3]==1||fields[3]==2||fields[3]==5)){world_policy=fields[3];phase=2;}else{phase=9;status=0;failure=reason;}}
   if(continuing){continue_refused=true;phase=9;status=0;failure=9;}
   else if(j.op==3||j.op==11){if(valid)phase=4;else{phase=9;status=0;failure=reason;}}
   else if(j.op==4){if(valid)phase=6;else{phase=9;status=0;failure=reason;}}
   else if(j.op==5){failure=reason;if(valid&&fields[3]<=2){status=fields[3];health_until=began+lease;}else if(!wrote||reason!=3){status=0;}}
   else if(j.op==7){report_pending=false;report_inflight=false;if(valid&&(fields[3]==1||fields[3]==2)){report_ack=j.sequence;status=fields[3];health_until=began+lease;}else{report_uncertain=wrote;status=0;}}
  }
  ReleaseSRWLockExclusive(&lock);WaitForSingleObject(wake,10);
 }
}
static BOOL CALLBACK start(PINIT_ONCE,void*,void**){wake=CreateEventW(nullptr,FALSE,FALSE,nullptr);if(!wake)return FALSE;HMODULE module=nullptr;if(!GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS|GET_MODULE_HANDLE_EX_FLAG_PIN,reinterpret_cast<LPCWSTR>(&run),&module)){CloseHandle(wake);return FALSE;}HANDLE thread=CreateThread(nullptr,0,run,nullptr,0,nullptr);if(!thread){CloseHandle(wake);return FALSE;}CloseHandle(thread);return TRUE;}
static int startup(uint32_t origin,uint8_t* out,uint32_t capacity,uint8_t* c,uint32_t budget){
 if(origin<1||origin>2||!out||capacity!=STARTUP_BYTES||!c||budget<1||budget>6000||!InitOnceExecuteOnce(&initialized,start,nullptr,nullptr))return 0;
 AcquireSRWLockExclusive(&lock);uint8_t any=0;for(uint32_t i=0;i<16;i++)any|=c[i];
 if(phase==0||(phase==9&&startup_latched&&any&&memcmp(c,context,16)!=0&&memcmp(c,startup_bootstrap,16)!=0)){
  if(any)memcpy(context,c,16);else if(BCryptGenRandom(nullptr,context,16,BCRYPT_USE_SYSTEM_PREFERRED_RNG)!=0){ReleaseSRWLockExclusive(&lock);return 0;}
  memcpy(startup_bootstrap,context,16);generation++;lobby_until=0;lobby_at=0;origins=1u<<origin;phase=1;status=0;world_policy=0;seed=0;authority_generation=0;startup_offered=false;startup_latched=true;bank_input.clear();bank_sequence=0;c1_input.clear();c1_sequence=0;debug_input.clear();debug_sequence=0;social_input.clear();social_input_sequence=0;social_ack=0;startup_began=GetTickCount64();startup_deadline=startup_began+budget;startup_next=0;startup_reason=0;startup_original=2;startup_attempts=0;startup_queue_ms=0;startup_claim_ms=0;startup_reply_ms=0;resuming=false;association_size=0;saved_record.clear();saved_lookup=4;control_pending=0;control_finished=0;controls.clear();memset(startup_record,0,sizeof(startup_record));report_pending=false;report_inflight=false;report_uncertain=false;report_sequence=0;report_ack=0;report_original=std::make_shared<const std::vector<uint8_t>>();pose_pending=false;peers_until=0;peers_rejected=false;social_until=0;SetEvent(wake);
 }else if(phase==9){memcpy(c,context,16);ReleaseSRWLockExclusive(&lock);return 0;}
 else if(any&&memcmp(c,context,16)!=0&&memcmp(c,startup_bootstrap,16)!=0){ReleaseSRWLockExclusive(&lock);return 0;}
 origins|=1u<<origin;memcpy(c,context,16);int result=phase==1?-3:phase>=2&&phase<9&&GetTickCount64()<health_until?1:0;
 if(phase==1&&GetTickCount64()>=startup_deadline){phase=9;status=0;if(!startup_reason)startup_reason=14;startup_original=0;result=0;}
 memcpy(out,startup_record,STARTUP_BYTES);if(result==1){uint32_t remaining=static_cast<uint32_t>(health_until-GetTickCount64());memcpy(out+32,&remaining,4);}else if(result==0){memset(out+12,0,4);memcpy(out+16,&startup_reason,4);}ReleaseSRWLockExclusive(&lock);return result;
}
static int prepare(uint32_t origin,uint32_t* out,uint8_t* c){if(!out)return 0;uint8_t record[STARTUP_BYTES];int result=startup(origin,record,STARTUP_BYTES,c,6000);if(result==1)memcpy(out,record+20,4);return result;}
static int startup_diagnostic(uint8_t* out,uint32_t capacity){if(!out||capacity!=256)return 0;AcquireSRWLockShared(&lock);memset(out,0,256);uint32_t values[12]={0x3154534e,1,startup_original,startup_reason,static_cast<uint32_t>(GetTickCount64()-startup_began),startup_queue_ms,startup_claim_ms,startup_reply_ms,startup_attempts,seed,world_policy,authority_generation};memcpy(out,values,sizeof(values));memcpy(out+48,context,16);memcpy(out+64,startup_record+64,80);ReleaseSRWLockShared(&lock);return 256;}
// Cached, nonblocking same-world admission. Only the verified Rust pipe reply
// can supply fresh authority; saved association/seed never authorize locally.
static int resume(uint32_t origin,const uint8_t* association,uint64_t actual,uint8_t* c){
 if(origin<1||origin>2||!association||!c||!InitOnceExecuteOnce(&initialized,start,nullptr,nullptr))return 0;
 uint8_t any=0;for(uint32_t i=0;i<16;i++)any|=association[i];if(!any)return 0;
 AcquireSRWLockExclusive(&lock);
 if(resuming&&memcmp(association,resume_association,16)==0&&actual==actual_seed&&phase!=9&&(phase==1||GetTickCount64()<health_until)){
  if((origins&(1u<<origin))&&memcmp(c,context,16)!=0){ReleaseSRWLockExclusive(&lock);return 0;}
  origins|=1u<<origin;memcpy(c,context,16);int result=phase==1?-3:1;ReleaseSRWLockExclusive(&lock);return result;
 }
 if((phase!=0&&phase!=9&&(phase==1||GetTickCount64()<health_until))||(resuming&&memcmp(c,context,16)==0)){ReleaseSRWLockExclusive(&lock);return 0;}
 if(BCryptGenRandom(nullptr,context,16,BCRYPT_USE_SYSTEM_PREFERRED_RNG)!=0){ReleaseSRWLockExclusive(&lock);return 0;}
 generation++;lobby_until=0;lobby_at=0;association_size=0;metadata_at=0;resume_decision=3;resume_retry_at=GetTickCount64()+100;resuming=true;memcpy(resume_association,association,16);actual_seed=actual;seed_bound=false;
 origins=1u<<origin;phase=1;status=0;health_until=0;authority_generation=0;report_ack=0;report_sequence=0;report_pending=false;report_inflight=false;report_uncertain=false;pose_sequence=0;pose_pending=false;peers_until=0;peers_rejected=false;social_until=0;social_ack=0;bank_input.clear();bank_sequence=0;c1_input.clear();c1_sequence=0;debug_input.clear();debug_sequence=0;social_input.clear();social_input_sequence=0;
 controls.clear();control_pending=0;control_finished=0;report_original=std::make_shared<const std::vector<uint8_t>>();memset(peers,0,864);memset(social,0,SOCIAL_BYTES);
 enqueue(12);controls.back().body.assign(association,association+16);const uint8_t* digits=reinterpret_cast<const uint8_t*>(&actual);controls.back().body.insert(controls.back().body.end(),digits,digits+8);
 memcpy(c,context,16);ReleaseSRWLockExclusive(&lock);return -3;
}
static int control(uint32_t origin,uint32_t op,const uint8_t* c,uint32_t payload){AcquireSRWLockExclusive(&lock);int result=0;
 if(active(origin,c)||(op==6&&origin>=1&&origin<=2&&c&&(origins&(1u<<origin))&&memcmp(c,context,16)==0)){
  if(op==6){bool retiring=(control_pending&(1u<<15))!=0;generation++;control_pending=0;control_finished=0;controls.clear();if(retiring)enqueue(15);if(startup_offered){enqueue(17);controls.back().body.assign(startup_record+64,startup_record+112);}else{enqueue(6);}phase=9;origins=0;status=0;health_until=0;pose_pending=false;peers_until=0;peers_rejected=false;social_until=0;bank_input.clear();bank_sequence=0;c1_input.clear();c1_sequence=0;debug_input.clear();debug_sequence=0;social_input.clear();social_input_sequence=0;result=1;}
  else if(op==3&&phase>=2&&phase<9&&(!seed_bound||actual_seed==payload)&&((world_policy==2)||((world_policy==1||world_policy==5)&&payload==seed))){if(phase>=4)result=1;else if(!(control_pending&((1u<<3)|(1u<<11)))){actual_seed=payload;seed_bound=true;enqueue(3,payload);}}
  else if(op==4&&phase>=4&&phase<9){if(phase>=6)result=1;else enqueue(4,payload);}
 }ReleaseSRWLockExclusive(&lock);return result;}
static int read_status(uint32_t origin,const uint8_t* c){AcquireSRWLockShared(&lock);int result=active(origin,c)&&GetTickCount64()<health_until?static_cast<int>(status):0;ReleaseSRWLockShared(&lock);return result;}
static int report(uint32_t origin,const uint8_t* c,const uint8_t* body,uint32_t size,uint64_t sequence){if(!body||!size||size>32768||!sequence)return 0;AcquireSRWLockExclusive(&lock);int result=0;
 if(active(origin,c)){bool same=sequence==report_sequence&&size==report_original->size()&&memcmp(body,report_original->data(),size)==0;
  if(same&&sequence==report_ack)result=1;
  // Inspection is an idempotent sequence-bound observation, not a mutation.
  // Resolve the exact original after a current held collection lease returns;
  // the app does not refresh observation freshness for a retransmission.
  else if(same&&!report_pending&&!report_inflight&&status>=1&&GetTickCount64()<health_until&&GetTickCount64()>=report_retry_at){report_job=job(7);report_job.sequence=sequence;report_job.original=report_original;report_pending=true;report_uncertain=false;report_retry_at=GetTickCount64()+250;SetEvent(wake);}
  else if(!report_pending&&!report_inflight&&!report_uncertain&&sequence>report_sequence&&bytes()-report_original->size()+size<=MAX_BYTES){report_original=std::make_shared<const std::vector<uint8_t>>(body,body+size);report_sequence=sequence;report_job=job(7);report_job.sequence=sequence;report_job.original=report_original;report_pending=true;SetEvent(wake);}else if(!same)refused++;
 }ReleaseSRWLockExclusive(&lock);return result;}
static int move(uint32_t origin,const uint8_t* c,const uint8_t* frame){if(!frame)return 0;AcquireSRWLockExclusive(&lock);uint32_t sequence=0;memcpy(&sequence,frame+24,4);bool ok=active(origin,c)&&status==2&&GetTickCount64()<health_until&&memcmp(frame,"NTG3\1\0\1\0",8)==0&&memcmp(frame+8,c,16)==0&&sequence>pose_sequence;
 ok=ok&&frame[44]<=1;for(uint32_t i=45;i<64;i++)ok=ok&&frame[i]==0;
 if(ok){if(pose_pending)coalesced++;memcpy(pose,frame,64);pose_sequence=sequence;pose_pending=true;SetEvent(wake);}ReleaseSRWLockExclusive(&lock);return ok?1:0;}
static int copy(uint32_t origin,const uint8_t* c,uint32_t kind,uint8_t* out,uint32_t capacity){if(!out)return kind==0?-1:0;
 // Saved Continue may be the first export in a fresh game process.
 // Start only the mailbox worker; never prepare/rearm a New Game here.
 if(kind==19&&!InitOnceExecuteOnce(&initialized,start,nullptr,nullptr))return 0;
 if(kind==7&&!InitOnceExecuteOnce(&initialized,start,nullptr,nullptr))return 4;
 AcquireSRWLockExclusive(&lock);int result=0;ULONGLONG now=GetTickCount64();
 if(kind==19&&origin>=1&&origin<=2&&capacity==256){lobby_requested=true;SetEvent(wake);if(now<lobby_until){memcpy(out,lobby_frame,256);result=256;}ReleaseSRWLockExclusive(&lock);return result;}
 if(kind==0){
  if(capacity!=864||!active(origin,c)||status!=2||now>=health_until||peers_rejected)result=-1;
  else if(now<peers_until){memcpy(out,peers,864);uint32_t remaining=static_cast<uint32_t>(peers_until-now);memcpy(out+24,&remaining,4);result=864;}
  ReleaseSRWLockExclusive(&lock);return result;
 }
 if(kind==7&&origin>=1&&origin<=2&&capacity>0&&capacity<=1024){
  bool same=saved_record.size()==capacity&&memcmp(saved_record.data(),out,capacity)==0;
  if(!same){saved_record.assign(out,out+capacity);saved_lookup=4;lookup_at=0;}
  if(now>=lookup_at&&!(control_pending&(1u<<14))){control_finished&=~(1u<<14);if(enqueue(14)){controls.back().body=saved_record;lookup_at=now+500;}}
  result=static_cast<int>(saved_lookup);ReleaseSRWLockExclusive(&lock);return result;
 }
 if(kind==9&&origin>=1&&origin<=2){if(!(control_pending&(1u<<15))){control_finished&=~(1u<<15);enqueue(15);}ReleaseSRWLockExclusive(&lock);return 1;}
 if(kind==8&&resuming&&c&&memcmp(c,context,16)==0){result=static_cast<int>(resume_decision);ReleaseSRWLockExclusive(&lock);return result;}
 if(active(origin,c)){
  if(kind==12&&capacity>0&&capacity<=4096&&debug_sequence<UINT64_MAX&&bytes()-debug_input.size()+capacity<=MAX_BYTES){debug_input.assign(out,out+capacity);debug_sequence++;ReleaseSRWLockExclusive(&lock);return 1;}
  if(kind==14&&status==1&&now<health_until&&now<social_until&&race_metadata_frame(social)&&*reinterpret_cast<uint32_t*>(social+24)==authority_generation&&capacity>=26&&capacity<=4096&&memcmp(out,"{\"kind\":\"race_completion\",",26)==0){if(c1_input.empty()&&c1_sequence<UINT64_MAX&&bytes()+capacity<=MAX_BYTES){c1_input.assign(out,out+capacity);c1_sequence++;SetEvent(wake);ReleaseSRWLockExclusive(&lock);return 1;}ReleaseSRWLockExclusive(&lock);return 0;}
  if(kind==14&&status==2&&now<health_until&&capacity>=24&&capacity<=4096&&(memcmp(out,"{\"kind\":\"c1_completion\",",24)==0||memcmp(out,"{\"kind\":\"c2_completion\",",24)==0||(capacity>=26&&memcmp(out,"{\"kind\":\"race_completion\",",26)==0))){if(c1_input.empty()&&c1_sequence<UINT64_MAX&&bytes()+capacity<=MAX_BYTES){c1_input.assign(out,out+capacity);c1_sequence++;SetEvent(wake);ReleaseSRWLockExclusive(&lock);return 1;}ReleaseSRWLockExclusive(&lock);return 0;}
  if(kind==14&&status==2&&now<health_until&&capacity>0&&capacity<=4096&&bank_input.empty()&&bank_sequence<UINT64_MAX&&bytes()+capacity<=MAX_BYTES){bank_input.assign(out,out+capacity);bank_sequence++;ReleaseSRWLockExclusive(&lock);return 1;}
  if(kind==15&&capacity==8192&&(status==2||(status==1&&race_metadata_frame(social)))&&now<health_until&&(social[5]==1||((social[5]==4||social[5]==5)&&*reinterpret_cast<uint32_t*>(social+24)==authority_generation))&&now<social_until){uint32_t n=0;memcpy(&n,social+32840,4);if(n>0&&n<=8192){memcpy(out,social+32848,n);ReleaseSRWLockExclusive(&lock);return static_cast<int>(n);}}
  if(kind==13&&capacity==1){out[0]=debug_enabled?1:0;ReleaseSRWLockExclusive(&lock);return 1;}
  if(kind==11&&status==2&&now<health_until&&capacity>0&&capacity<=4096&&social_input.empty()&&social_input_sequence<UINT64_MAX&&bytes()+capacity<=MAX_BYTES){social_input.assign(out,out+capacity);social_input_sequence++;SetEvent(wake);result=1;}
  if(kind==10&&capacity==1&&display_generation==generation&&now<display_until&&now<health_until&&display_epoch==authority_generation){result=static_cast<int>(display_code);}
  else if(kind==6&&capacity==1024&&association_size){memcpy(out,association_record,association_size);result=static_cast<int>(association_size);}
  else if(kind==5&&capacity==8&&phase>=2&&now<health_until){uint64_t actual=0;memcpy(&actual,out,8);if((!seed_bound||actual==actual_seed)&&(world_policy==2||((world_policy==1||world_policy==5)&&actual==seed))){if(phase>=4)result=1;else if(!(control_pending&((1u<<3)|(1u<<11)))){actual_seed=actual;seed_bound=true;if(enqueue(11,static_cast<uint32_t>(actual)))controls.back().body.assign(out,out+8);}}}
  else if(kind==4&&capacity==4&&phase>=2&&now<health_until&&world_policy){memcpy(out,&world_policy,4);result=4;}
  else if(kind==1&&capacity==64&&status==2&&now<health_until&&social[5]==1&&now<social_until){memcpy(out,social,64);uint32_t remaining=static_cast<uint32_t>(social_until-now);memcpy(out+28,&remaining,4);result=64;}
  else if((kind==2||kind==3)&&capacity==32768&&now<social_until){uint32_t size=0;memcpy(&size,social+32,4);bool terminal=kind==3&&social[5]==2&&size==41&&social[104]==6;
   if(size<=32768&&size>0&&(terminal||(kind==2&&status==2&&now<health_until&&social[5]==1))){memcpy(out,social+64,size);memcpy(&social_ack,social+92,4);memset(social+32,0,4);result=static_cast<int>(size);}}
 }ReleaseSRWLockExclusive(&lock);return result;}
static int diagnostic(uint32_t origin,const uint8_t* c){AcquireSRWLockShared(&lock);int result=active(origin,c)?static_cast<int>(failure):7;ReleaseSRWLockShared(&lock);return result;}
static int metrics(uint32_t* out,uint32_t count){if(!out)return 0;
#ifdef NT2_IPC_PRIVATE
 if(count==24){if(!metrics(out,9))return 0;uint64_t values[15]={};nt2_local_ws_counters(values,15);for(uint32_t i=0;i<15;i++)out[9+i]=static_cast<uint32_t>(values[i]>UINT32_MAX?UINT32_MAX:values[i]);return 1;}
#endif
 if(count!=9)return 0;AcquireSRWLockShared(&lock);out[0]=worker_id;out[1]=completed[0];out[2]=completed[1];out[3]=completed[2];out[4]=completed[3];out[5]=coalesced;out[6]=refused;out[7]=bytes();out[8]=max_bytes;ReleaseSRWLockShared(&lock);return 1;}
static const NT2IpcApi api={3,sizeof(NT2IpcApi),prepare,control,read_status,report,move,copy,diagnostic,metrics,resume,startup,startup_diagnostic};
extern "C" __declspec(dllexport) const NT2IpcApi* __cdecl nt2_g3_ipc_api(uint32_t version){return version==3?&api:nullptr;}

