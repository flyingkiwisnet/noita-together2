// First-party Windows/x86 G0 bridge. No executable patching, hooks, tokens or arbitrary paths.
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <sddl.h>
#include <bcrypt.h>
#include <stdint.h>
#include <string>
#include <vector>
#include <cstring>
#ifdef NT2_TEST_INITIAL_TRACE
#include <cstdio>
static int initial_trace(uint32_t op,const char* stage,int result){
 std::fprintf(stderr,"NT2_PRIVATE_CONTACT op=%u stage=%s result=%d\n",op,stage,result);return result;
}
#else
static int initial_trace(uint32_t,const char*,int result){return result;}
#endif
#pragma comment(lib,"advapi32.lib")
#pragma comment(lib,"bcrypt.lib")
#ifndef NT2_PIPE_PREFIX
#define NT2_PIPE_PREFIX L"\\\\.\\pipe\\nt2-g2-v1-"
#endif
#ifndef NT2_APP_HINT_SUFFIX
#define NT2_APP_HINT_SUFFIX L"\\NoitaTogether2\\bridge-app.txt"
#endif
static uint8_t nonce[16];static uint32_t seed=0;static bool attached=false;
struct Liveness {HANDLE stop;uint8_t context[16];volatile LONG status;volatile LONG failure;volatile LONG64 updated;};
static HANDLE worker=nullptr;static Liveness* liveness=nullptr;
static std::wstring sid(){HANDLE token=nullptr;if(!OpenProcessToken(GetCurrentProcess(),TOKEN_QUERY,&token))return L"";DWORD n=0;GetTokenInformation(token,TokenUser,nullptr,0,&n);std::vector<uint8_t> b(n);if(!GetTokenInformation(token,TokenUser,b.data(),n,&n)){CloseHandle(token);return L"";}CloseHandle(token);LPWSTR text=nullptr;if(!ConvertSidToStringSidW(((TOKEN_USER*)b.data())->User.Sid,&text))return L"";std::wstring result(text);LocalFree(text);return result;}
static bool app(HANDLE pipe){
#ifdef NT2_TEST_APP_PATH
 ULONG pid=0;if(!GetNamedPipeServerProcessId(pipe,&pid))return false;HANDLE process=OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION,FALSE,pid);if(!process)return false;wchar_t image[1024];DWORD count=1024;bool ok=QueryFullProcessImageNameW(process,0,image,&count)!=0;CloseHandle(process);return ok&&_wcsicmp(image,NT2_TEST_APP_PATH)==0;
#else
 wchar_t local[1024];DWORD size=GetEnvironmentVariableW(L"LOCALAPPDATA",local,1024);if(size==0||size>=1024)return false;
 std::wstring file=std::wstring(local)+NT2_APP_HINT_SUFFIX;
 HANDLE input=CreateFileW(file.c_str(),GENERIC_READ,FILE_SHARE_READ,nullptr,OPEN_EXISTING,FILE_ATTRIBUTE_NORMAL,nullptr);if(input==INVALID_HANDLE_VALUE)return false;
 char bytes[2049]={};DWORD n=0;bool read=ReadFile(input,bytes,2048,&n,nullptr)!=0;LARGE_INTEGER length={};bool bounded=GetFileSizeEx(input,&length)&&length.QuadPart>0&&length.QuadPart<=2048;CloseHandle(input);if(!read||!bounded||n!=length.QuadPart||memchr(bytes,0,n))return false;
 wchar_t expected[1024];int chars=MultiByteToWideChar(CP_UTF8,MB_ERR_INVALID_CHARS,bytes,(int)n,expected,1023);if(chars<3||expected[1]!=L':'||expected[2]!=L'\\')return false;expected[chars]=0;
 ULONG pid=0;if(!GetNamedPipeServerProcessId(pipe,&pid))return false;HANDLE process=OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION,FALSE,pid);if(!process)return false;wchar_t image[1024];DWORD count=1024;bool ok=QueryFullProcessImageNameW(process,0,image,&count)!=0;CloseHandle(process);return ok&&_wcsicmp(image,expected)==0;
#endif
}
static bool transfer(HANDLE h,bool write,void* bytes,DWORD size,bool* timed_out=nullptr,DWORD wait_ms=80){OVERLAPPED o={};o.hEvent=CreateEventW(nullptr,TRUE,FALSE,nullptr);if(!o.hEvent)return false;DWORD n=0;BOOL ok=write?WriteFile(h,bytes,size,&n,&o):ReadFile(h,bytes,size,&n,&o);if(!ok&&GetLastError()==ERROR_IO_PENDING){DWORD waited=WaitForSingleObject(o.hEvent,wait_ms);if(waited==WAIT_OBJECT_0)ok=GetOverlappedResult(h,&o,&n,FALSE);else{if(timed_out&&waited==WAIT_TIMEOUT)*timed_out=true;CancelIoEx(h,&o);GetOverlappedResult(h,&o,&n,TRUE);ok=FALSE;}}CloseHandle(o.hEvent);return ok&&n==size;}
static int exchange_context(uint32_t op,const uint8_t context[16],uint32_t* result=nullptr,uint32_t payload=0,bool* timed_out=nullptr,uint32_t* failure=nullptr){
 if(timed_out)*timed_out=false;if(failure)*failure=0;auto finish=[&](uint32_t reason,const char* stage,int value){if(failure)*failure=reason;return initial_trace(op,stage,value);};std::wstring user=sid();if(user.empty())return finish(8,"sid-unavailable",0);
 DWORD bootstrap_wait=80;
#ifdef NT2_G3_BOOTSTRAP_WAIT
 // One exact initial exchange, not a retry or a playing-health lease.
 // Durable claim sync and single-instance handover may outlast80ms.
 if(op==1||op==3||op==4)bootstrap_wait=250;
#endif
 std::wstring path=NT2_PIPE_PREFIX+user;
#ifdef NT2_TEST_INITIAL_TRACE
 // Private diagnostic only: force availability to be taken between wait/open.
 ULONGLONG connect_begin=GetTickCount64();char exhaust_flag[2]={};
 bool exhaust_connection=op==5&&GetEnvironmentVariableA("NT2_HEARTBEAT_CONNECT_EXHAUST",exhaust_flag,2)==1&&exhaust_flag[0]=='1';
 char race_flag[2]={};bool inject_race=(op==4&&GetEnvironmentVariableA("NT2_INITIAL_CONNECT_RACE",race_flag,2)==1&&race_flag[0]=='1')
  ||(op==5&&GetEnvironmentVariableA("NT2_HEARTBEAT_CONNECT_RACE",race_flag,2)==1&&race_flag[0]=='1')||exhaust_connection;
 HANDLE race_initial=inject_race?CreateFileW(path.c_str(),GENERIC_READ|GENERIC_WRITE,0,nullptr,OPEN_EXISTING,FILE_FLAG_OVERLAPPED,nullptr):INVALID_HANDLE_VALUE;
#endif
 HANDLE pipe=CreateFileW(path.c_str(),GENERIC_READ|GENERIC_WRITE,0,nullptr,OPEN_EXISTING,FILE_FLAG_OVERLAPPED,nullptr);
 bool busy=pipe==INVALID_HANDLE_VALUE&&GetLastError()==ERROR_PIPE_BUSY;
#ifdef NT2_TEST_INITIAL_TRACE
 if(race_initial!=INVALID_HANDLE_VALUE&&!exhaust_connection)CloseHandle(race_initial);
 if(exhaust_connection)inject_race=false;
#endif
#ifdef NT2_G3_BOOTSTRAP_WAIT
 if(bootstrap_wait==250||op==5){
  // WaitNamedPipe availability is not reserved: another existing worker can win.
  // Reacquire only BEFORE any request write, inside one original connection budget.
  // G3 heartbeat uses the same bounded reacquisition with its original80ms budget.
  ULONGLONG until=GetTickCount64()+bootstrap_wait;
  while(busy){
   ULONGLONG now=GetTickCount64();if(now>=until)break;
   DWORD remaining=static_cast<DWORD>(until-now);
   if(!WaitNamedPipeW(path.c_str(),remaining)||GetTickCount64()>=until)break;
#ifdef NT2_TEST_INITIAL_TRACE
   HANDLE competitor=inject_race?CreateFileW(path.c_str(),GENERIC_READ|GENERIC_WRITE,0,nullptr,OPEN_EXISTING,FILE_FLAG_OVERLAPPED,nullptr):INVALID_HANDLE_VALUE;
   // Keep trying the private witness if a normal worker won availability first.
   if(competitor!=INVALID_HANDLE_VALUE)inject_race=false;
#endif
   pipe=CreateFileW(path.c_str(),GENERIC_READ|GENERIC_WRITE,0,nullptr,OPEN_EXISTING,FILE_FLAG_OVERLAPPED,nullptr);
   busy=pipe==INVALID_HANDLE_VALUE&&GetLastError()==ERROR_PIPE_BUSY;
#ifdef NT2_TEST_INITIAL_TRACE
   if(competitor!=INVALID_HANDLE_VALUE){initial_trace(op,"test-availability-race-injected",0);CloseHandle(competitor);}
#endif
   if(busy){initial_trace(op,"pipe-availability-raced",0);Sleep(1);}
  }
 }else
#endif
#ifdef NT2_TEST_INITIAL_TRACE
 if(busy&&WaitNamedPipeW(path.c_str(),bootstrap_wait)){
  HANDLE competitor=inject_race?CreateFileW(path.c_str(),GENERIC_READ|GENERIC_WRITE,0,nullptr,OPEN_EXISTING,FILE_FLAG_OVERLAPPED,nullptr):INVALID_HANDLE_VALUE;
  pipe=CreateFileW(path.c_str(),GENERIC_READ|GENERIC_WRITE,0,nullptr,OPEN_EXISTING,FILE_FLAG_OVERLAPPED,nullptr);
  if(competitor!=INVALID_HANDLE_VALUE){initial_trace(op,"test-availability-race-injected",0);CloseHandle(competitor);}
 }
#else
 if(busy&&WaitNamedPipeW(path.c_str(),bootstrap_wait))pipe=CreateFileW(path.c_str(),GENERIC_READ|GENERIC_WRITE,0,nullptr,OPEN_EXISTING,FILE_FLAG_OVERLAPPED,nullptr);
#endif
#ifdef NT2_TEST_INITIAL_TRACE
 if(exhaust_connection&&race_initial!=INVALID_HANDLE_VALUE){initial_trace(op,"test-connection-exhausted",0);CloseHandle(race_initial);}
 if(op==5)std::fprintf(stderr,"NT2_PRIVATE_CONNECT op=5 budget_ms=%lu elapsed_ms=%llu\n",(unsigned long)bootstrap_wait,GetTickCount64()-connect_begin);
#endif
 if(pipe==INVALID_HANDLE_VALUE)return finish(busy?6:7,busy?"pipe-busy":"pipe-unavailable",0);
 if(!app(pipe)){CloseHandle(pipe);return finish(2,"app-identity-refused",0);}
 uint8_t request[32]={};uint32_t fields[4]={0x3247544e,2,op,payload};memcpy(request,fields,16);memcpy(request+16,context,16);uint8_t reply[32]={};
 bool timeout=false;bool wrote=transfer(pipe,true,request,32,&timeout);bool read=wrote&&transfer(pipe,false,reply,32,&timeout,bootstrap_wait);
 if(timed_out)*timed_out=timeout;CloseHandle(pipe);
 if(!wrote)return finish(timeout?3:4,timeout?"verified-write-timeout":"verified-write-failed",0);
 if(!read)return finish(timeout?3:4,timeout?"verified-read-timeout":"verified-read-failed",0);
 uint32_t values[4];memcpy(values,reply,16);
 if(values[0]!=0x3247544e||values[1]!=2)return finish(5,"invalid-reply",0);
 if(values[2]!=1)return finish(9,"negative-reply",0);
 if(memcmp(reply+16,context,16)!=0)return finish(5,"invalid-reply",0);
 if(result)*result=values[3];return finish(0,"acknowledged",1);
}
static DWORD WINAPI heartbeat(void* raw){Liveness* state=static_cast<Liveness*>(raw);while(WaitForSingleObject(state->stop,2000)==WAIT_TIMEOUT){uint32_t status=0;if(WaitForSingleObject(state->stop,0)!=WAIT_TIMEOUT)break;bool timed_out=false;uint32_t failure=0;if(exchange_context(5,state->context,&status,0,&timed_out,&failure)){InterlockedExchange(&state->status,(LONG)status);InterlockedExchange64(&state->updated,(LONG64)GetTickCount64());}else {
#ifdef NT2_G3_VERIFIED_HEARTBEAT_TIMEOUT
 // A timed-out I/O to the verified app is not a negative authority reply.
 // Retain only its existing cached status/age; never refresh the12s deadline.
 if(!timed_out)InterlockedExchange(&state->status,0);
#else
 InterlockedExchange(&state->status,0);
#endif
 }InterlockedExchange(&state->failure,(LONG)failure);}return 0;}static bool stop_liveness(){if(!worker)return true;SetEvent(liveness->stop);if(WaitForSingleObject(worker,400)!=WAIT_OBJECT_0)return false;CloseHandle(worker);CloseHandle(liveness->stop);delete liveness;worker=nullptr;liveness=nullptr;return true;}
static bool start_liveness(){HMODULE module=nullptr;if(!GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS|GET_MODULE_HANDLE_EX_FLAG_PIN,reinterpret_cast<LPCWSTR>(&heartbeat),&module))return false;liveness=new Liveness{};liveness->stop=CreateEventW(nullptr,TRUE,FALSE,nullptr);if(!liveness->stop){delete liveness;liveness=nullptr;return false;}memcpy(liveness->context,nonce,16);liveness->status=1;liveness->updated=(LONG64)GetTickCount64();worker=CreateThread(nullptr,0,heartbeat,liveness,0,nullptr);if(!worker){CloseHandle(liveness->stop);delete liveness;liveness=nullptr;return false;}return true;}
static bool saved_run(){
#ifdef NT2_TEST_NOSAVE
return false; // Isolated test DLL only; production build never sets this flag.
#else
wchar_t appdata[1024];DWORD n=GetEnvironmentVariableW(L"APPDATA",appdata,1024);if(n==0||n>=1024)return true;std::wstring root(appdata);size_t split=root.find_last_of(L"\\/");if(split==std::wstring::npos)return true;root=root.substr(0,split)+L"\\LocalLow\\Nolla_Games_Noita\\save00\\";return GetFileAttributesW((root+L"world_state.xml").c_str())!=INVALID_FILE_ATTRIBUTES||GetFileAttributesW((root+L"player.xml").c_str())!=INVALID_FILE_ATTRIBUTES;
#endif
}
// Every export is bound to the immutable Lua-world token. An old finalizer cannot close a new worker.
static bool current(const uint8_t* context){return attached&&context&&memcmp(context,nonce,16)==0;}
extern "C" __declspec(dllexport) int __cdecl nt2_g2_prepare(uint32_t* out,uint8_t* context){
 if(!out||!context)return 0;if(!stop_liveness()){attached=false;exchange_context(6,nonce);return 0;}if(attached)exchange_context(6,nonce);attached=false;
 if(BCryptGenRandom(nullptr,nonce,16,BCRYPT_USE_SYSTEM_PREFERRED_RNG)!=0)return 0;if(saved_run()){exchange_context(2,nonce);return -2;}
 if(!exchange_context(1,nonce,&seed))return 0;attached=true;if(!start_liveness()){exchange_context(6,nonce);attached=false;return 0;}*out=seed;memcpy(context,nonce,16);return 1;
}
extern "C" __declspec(dllexport) int __cdecl nt2_g2_seed_accepted(const uint8_t* context,uint32_t actual){return current(context)&&actual==seed?exchange_context(3,context,nullptr,actual):0;}
extern "C" __declspec(dllexport) int __cdecl nt2_g2_world_loaded(const uint8_t* context){return current(context)?exchange_context(4,context,nullptr,1):0;}
extern "C" __declspec(dllexport) int __cdecl nt2_g2_status(const uint8_t* context){if(!current(context)||!liveness)return 0;if(GetTickCount64()-(ULONGLONG)InterlockedCompareExchange64(&liveness->updated,0,0)>12000)return 0;return (int)InterlockedCompareExchange(&liveness->status,0,0);}
extern "C" __declspec(dllexport) int __cdecl nt2_g2_close(const uint8_t* context){if(!current(context))return 0;if(!stop_liveness()){exchange_context(6,context);attached=false;return 0;}exchange_context(6,context);attached=false;return 1;}
