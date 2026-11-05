#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <bcrypt.h>
#pragma comment(lib,"bcrypt.lib")
#include "abi.h"
#include "../g3-bridge/frame.cpp"
#ifndef NT2_IPC_ORIGIN
#error Explicit primary/social origin required
#endif
static INIT_ONCE loader=INIT_ONCE_STATIC_INIT;static const NT2IpcApi* api=nullptr;static HANDLE loader_done=nullptr;
static BOOL CALLBACK load(PINIT_ONCE,void*,void**){
 HMODULE own=nullptr;if(!GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS|GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,reinterpret_cast<LPCWSTR>(&load),&own))return FALSE;
 wchar_t path[32768];DWORD length=GetModuleFileNameW(own,path,32768);if(!length||length>=32768)return FALSE;
 wchar_t* slash=wcsrchr(path,L'\\');if(!slash)return FALSE;*(slash+1)=0;
 if(wcslen(path)+wcslen(L"nt2_g3_ipc.dll")>=32768)return FALSE;wcscat_s(path,L"nt2_g3_ipc.dll");
 DWORD flags=GetFileAttributesW(path);if(flags==INVALID_FILE_ATTRIBUTES||(flags&(FILE_ATTRIBUTE_DIRECTORY|FILE_ATTRIBUTE_REPARSE_POINT)))return FALSE;
 HMODULE broker=LoadLibraryExW(path,nullptr,LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR|LOAD_LIBRARY_SEARCH_DEFAULT_DIRS);if(!broker)return FALSE;
 auto get=reinterpret_cast<NT2IpcGetApi>(GetProcAddress(broker,"nt2_g3_ipc_api"));
 const NT2IpcApi* found=get?get(3):nullptr;if(!found||found->version!=3||found->size!=sizeof(NT2IpcApi)){FreeLibrary(broker);return FALSE;}api=found;return TRUE;
}
static DWORD WINAPI load_worker(void*){
#ifdef NT2_TEST_LOADER_DELAY_MS
 Sleep(NT2_TEST_LOADER_DELAY_MS); // Private fault injection, same real loader.
#endif
 load(nullptr,nullptr,nullptr);SetEvent(loader_done);return 0;
}
static BOOL CALLBACK start_loader(PINIT_ONCE,void*,void**){
 loader_done=CreateEventW(nullptr,TRUE,FALSE,nullptr);if(!loader_done)return FALSE;
 // One bounded worker per wrapper. Its module must outlive a timed-out caller
 // and Lua namespace collection. At most the two fixed wrapper images are pinned.
 HMODULE own=nullptr;if(!GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS|GET_MODULE_HANDLE_EX_FLAG_PIN,reinterpret_cast<LPCWSTR>(&start_loader),&own)){CloseHandle(loader_done);loader_done=nullptr;return FALSE;}
 HANDLE worker=CreateThread(nullptr,0,load_worker,nullptr,0,nullptr);if(!worker){CloseHandle(loader_done);loader_done=nullptr;return FALSE;}CloseHandle(worker);return TRUE;
}
static const NT2IpcApi* broker(){if(!InitOnceExecuteOnce(&loader,start_loader,nullptr,nullptr)||WaitForSingleObject(loader_done,0)!=WAIT_OBJECT_0)return nullptr;return api;}
extern "C" __declspec(dllexport) int __cdecl nt2_g3_prepare(uint32_t* out,uint8_t* context){auto a=broker();return a?a->prepare(NT2_IPC_ORIGIN,out,context):0;}
// Development complete transaction. Only the pre-generation caller waits;
// frame exports remain cached. A timeout is a failure, never Continue evidence.
static uint8_t startup_latched[256]={},startup_latched_context[16]={};static bool startup_has_latch=false,startup_loader_refused=false;
// Non-authorizing local correlation only. No broker, pipe, claim or loader wait.
// The settings adapter consumes its engine entry evidence before requesting it.
extern "C" __declspec(dllexport) int __cdecl nt2_g3_startup_nonce(uint8_t* out,uint32_t capacity){
 if(!out||capacity!=16)return 0;
 if(BCryptGenRandom(nullptr,out,16,BCRYPT_USE_SYSTEM_PREFERRED_RNG)!=0){memset(out,0,16);return 0;}
 uint8_t any=0;for(uint32_t i=0;i<16;i++)any|=out[i];return any?1:0;
}
extern "C" __declspec(dllexport) int __cdecl nt2_g3_startup_bundle(uint8_t* out,uint32_t capacity,uint8_t* context,uint32_t budget){
 if(!out||capacity!=16384||!context||budget<100||budget>6000)return 0;
 if(startup_loader_refused){uint8_t any=0;for(uint32_t i=0;i<16;i++)any|=context[i];if(!any||memcmp(startup_latched_context,context,16)==0){memset(out,0,capacity);uint32_t fields[5]={0x3247544e,9,16,0,0};memcpy(fields+4,startup_latched+12,4);memcpy(out,fields,sizeof(fields));memcpy(out+48,context,16);return 0;}startup_loader_refused=false;}
 uint8_t measured[256]={};uint32_t diagnostic_header[2]={0x3154534e,1};memcpy(measured,diagnostic_header,8);ULONGLONG began=GetTickCount64();auto a=broker();
 bool loader_timed_out=false;if(!a&&loader_done){uint32_t elapsed=static_cast<uint32_t>(GetTickCount64()-began);if(elapsed<budget){DWORD waited=WaitForSingleObject(loader_done,budget-elapsed);if(waited==WAIT_OBJECT_0)a=broker();else loader_timed_out=waited==WAIT_TIMEOUT;}else loader_timed_out=true;}
 uint32_t loader_ms=static_cast<uint32_t>(GetTickCount64()-began);int result=0;
 if(a&&loader_ms<budget){result=a->startup(NT2_IPC_ORIGIN,out,capacity,context,budget-loader_ms);while(result==-3&&GetTickCount64()-began<budget){Sleep(1);uint32_t left=budget-static_cast<uint32_t>(GetTickCount64()-began);result=a->startup(NT2_IPC_ORIGIN,out,capacity,context,left?left:1);}a->startup_diagnostic(measured,256);}
 else{uint32_t reason=20;memcpy(measured+12,&reason,4);memset(out,0,capacity);uint32_t fields[5]={0x3247544e,9,16,0,20};memcpy(out,fields,sizeof(fields));memcpy(out+48,context,16);startup_loader_refused=true;}
 uint32_t elapsed=static_cast<uint32_t>(GetTickCount64()-began);memcpy(measured+144,&loader_ms,4);memcpy(measured+148,&elapsed,4);memcpy(measured+152,&result,4);uint32_t known=0;if(a){uint32_t state=0;memcpy(&state,measured+8,4);known=state==1?1:0;}memcpy(measured+156,&known,4);if(result==-3||elapsed>=budget||loader_timed_out){uint32_t reason=14;memcpy(measured+12,&reason,4);}if(!startup_has_latch||memcmp(startup_latched_context,context,16)!=0){memcpy(startup_latched,measured,256);memcpy(startup_latched_context,context,16);startup_has_latch=true;}
 if(result==-3||elapsed>=budget||loader_timed_out){uint32_t reason=14;memset(out+12,0,4);memcpy(out+16,&reason,4);if(a)a->control(NT2_IPC_ORIGIN,6,context,0);return 0;}return result;
}
extern "C" __declspec(dllexport) int __cdecl nt2_g3_startup_diagnostic(uint8_t* out,uint32_t capacity){if(!out||capacity!=256)return 0;memcpy(out,startup_latched,256);return 256;}
extern "C" __declspec(dllexport) int __cdecl nt2_g3_startup_prepare(uint32_t* out,uint8_t* context){if(!out)return 0;uint8_t bundle[16384];int result=nt2_g3_startup_bundle(bundle,sizeof(bundle),context,6000);if(result==1)memcpy(out,bundle+20,4);return result;}
extern "C" __declspec(dllexport) int __cdecl nt2_g3_base_prepare(uint32_t* out,uint8_t* context){return nt2_g3_prepare(out,context);}
extern "C" __declspec(dllexport) int __cdecl nt2_g3_seed_accepted(const uint8_t* context,uint32_t actual){auto a=broker();return a?a->control(NT2_IPC_ORIGIN,3,context,actual):0;}
extern "C" __declspec(dllexport) int __cdecl nt2_g3_world_loaded(const uint8_t* context){auto a=broker();return a?a->control(NT2_IPC_ORIGIN,4,context,1):0;}
extern "C" __declspec(dllexport) int __cdecl nt2_g3_status(const uint8_t* context){auto a=broker();return a?a->status(NT2_IPC_ORIGIN,context):0;}
// Optional display-only cached poll; no enqueue/I/O and no authority effect.
extern "C" __declspec(dllexport) int __cdecl nt2_g3_lobby_status(uint8_t* out,uint32_t capacity){auto a=broker();return a?a->copy(NT2_IPC_ORIGIN,nullptr,19,out,capacity):0;}
extern "C" __declspec(dllexport) int __cdecl nt2_g3_display_status(const uint8_t* context){uint8_t unused=0;auto a=broker();return a?a->copy(NT2_IPC_ORIGIN,context,10,&unused,1):0;}
extern "C" __declspec(dllexport) int __cdecl nt2_g3_close(const uint8_t* context){auto a=broker();return a?a->control(NT2_IPC_ORIGIN,6,context,0):0;}
extern "C" __declspec(dllexport) int __cdecl nt2_g3_base_close(const uint8_t* context){return nt2_g3_close(context);}
extern "C" __declspec(dllexport) int __cdecl nt2_g3_heartbeat_diagnostic(const uint8_t* context){auto a=broker();return a?a->diagnostic(NT2_IPC_ORIGIN,context):7;}
extern "C" __declspec(dllexport) int __cdecl nt2_g3_report(const uint8_t* context,const uint8_t* body,uint32_t size,uint64_t sequence){auto a=broker();return a?a->report(NT2_IPC_ORIGIN,context,body,size,sequence):0;}
extern "C" __declspec(dllexport) int __cdecl nt2_g3_move(const uint8_t* context,uint32_t sequence,int32_t x,int32_t y,int32_t vx,int32_t vy,uint32_t facing){auto a=broker();uint8_t frame[64];return a&&NT2G3EncodePose(frame,64,context,sequence,x,y,vx,vy,facing)==64?a->move(NT2_IPC_ORIGIN,context,frame):0;}
extern "C" __declspec(dllexport) int __cdecl nt2_g3_peers(const uint8_t* context,uint8_t* out,uint32_t size){auto a=broker();return a?a->copy(NT2_IPC_ORIGIN,context,0,out,size):-1;}
#ifdef NT2_IPC_SOCIAL
extern "C" __declspec(dllexport) int __cdecl nt2_g3_social_lease(const uint8_t* context,uint8_t* out,uint32_t size){auto a=broker();return a?a->copy(NT2_IPC_ORIGIN,context,1,out,size):0;}
extern "C" __declspec(dllexport) int __cdecl nt2_g3_social_next(const uint8_t* context,uint8_t* out,uint32_t size){auto a=broker();return a?a->copy(NT2_IPC_ORIGIN,context,2,out,size):0;}
extern "C" __declspec(dllexport) int __cdecl nt2_g3_social_terminal(const uint8_t* context,uint8_t* out,uint32_t size){auto a=broker();return a?a->copy(NT2_IPC_ORIGIN,context,3,out,size):0;}
#endif
#ifdef NT2_IPC_PRIVATE
extern "C" __declspec(dllexport) const NT2IpcApi* __cdecl nt2_g3_private_api(){return broker();}
#endif


// Cached exact world policy; never waits or creates another broker job.
extern "C" __declspec(dllexport) uint32_t __cdecl nt2_g3_world_policy(const uint8_t* context){auto a=broker();uint32_t policy=0;return a&&a->copy(NT2_IPC_ORIGIN,context,4,reinterpret_cast<uint8_t*>(&policy),4)==4?policy:0;}

// Canonical actual seeds are decimal uint64; no Lua double rounding/truncation.
extern "C" __declspec(dllexport) int __cdecl nt2_g3_seed_known(const uint8_t* context,const uint8_t* digits,uint32_t size){
 if(!digits||!size||size>20||(size>1&&digits[0]=='0'))return 0;uint64_t actual=0;
 for(uint32_t i=0;i<size;i++){if(digits[i]<'0'||digits[i]>'9')return 0;uint64_t digit=digits[i]-'0';if(actual>(UINT64_MAX-digit)/10)return 0;actual=actual*10+digit;}
 auto a=broker();return a?a->copy(NT2_IPC_ORIGIN,context,5,reinterpret_cast<uint8_t*>(&actual),8):0;
}

// Resume never calls SetWorldSeed and has no startup wait. Frame calls only poll.
extern "C" __declspec(dllexport) int __cdecl nt2_g3_resume(const uint8_t* association,const uint8_t* digits,uint32_t size,uint8_t* context){
 if(!digits||!size||size>20||(size>1&&digits[0]=='0'))return 0;uint64_t actual=0;
 for(uint32_t i=0;i<size;i++){if(digits[i]<'0'||digits[i]>'9')return 0;uint64_t d=digits[i]-'0';if(actual>(UINT64_MAX-d)/10)return 0;actual=actual*10+d;}
 auto a=broker();return a?a->resume(NT2_IPC_ORIGIN,association,actual,context):0;
}

// Non-secret metadata and choices only. All calls copy/poll the same broker.
extern "C" __declspec(dllexport) int __cdecl nt2_g3_association(const uint8_t* c,uint8_t* out,uint32_t size){auto a=broker();return a?a->copy(NT2_IPC_ORIGIN,c,6,out,size):0;}
extern "C" __declspec(dllexport) int __cdecl nt2_g3_saved_lookup(const uint8_t* record,uint32_t size){if(!record||!size||size>1024)return 0;uint8_t copy[1024];memcpy(copy,record,size);auto a=broker();return a?a->copy(NT2_IPC_ORIGIN,nullptr,7,copy,size):0;}
extern "C" __declspec(dllexport) int __cdecl nt2_g3_resume_decision(const uint8_t* c){uint8_t unused=0;auto a=broker();return a?a->copy(NT2_IPC_ORIGIN,c,8,&unused,1):0;}
extern "C" __declspec(dllexport) int __cdecl nt2_g3_offline_choice(){uint8_t unused=0;auto a=broker();return a?a->copy(NT2_IPC_ORIGIN,nullptr,9,&unused,1):0;}
extern "C" __declspec(dllexport) int __cdecl nt2_g3_social_publish(const uint8_t* c,const uint8_t* bytes,uint32_t size){if(!c||!bytes||!size||size>4096)return 0;uint8_t copy[4096];memcpy(copy,bytes,size);auto a=broker();return a?a->copy(NT2_IPC_ORIGIN,c,11,copy,size):0;}

// Optional, coalesced diagnostic copy; never a gameplay original or play grant.
extern "C" __declspec(dllexport) int __cdecl nt2_g3_debug_publish(const uint8_t* c,const uint8_t* bytes,uint32_t size){if(!c||!bytes||!size||size>4096)return 0;uint8_t copy[4096];memcpy(copy,bytes,size);auto a=broker();return a?a->copy(NT2_IPC_ORIGIN,c,12,copy,size):0;}
extern "C" __declspec(dllexport) int __cdecl nt2_g3_debug_mode(const uint8_t* c){uint8_t value=0;auto a=broker();return a&&a->copy(NT2_IPC_ORIGIN,c,13,&value,1)==1?value:-1;}

// Independent bounded bank mailbox, carried on existing optional polls.
extern "C" __declspec(dllexport) int __cdecl nt2_g3_bank_publish(const uint8_t* c,const uint8_t* bytes,uint32_t size){if(!c||!bytes||!size||size>4096)return 0;uint8_t data[4096];memcpy(data,bytes,size);auto a=broker();return a?a->copy(NT2_IPC_ORIGIN,c,14,data,size):0;}
extern "C" __declspec(dllexport) int __cdecl nt2_g3_bank_view(const uint8_t* c,uint8_t* out,uint32_t size){if(!c||!out||size!=8192)return 0;auto a=broker();return a?a->copy(NT2_IPC_ORIGIN,c,15,out,size):0;}
