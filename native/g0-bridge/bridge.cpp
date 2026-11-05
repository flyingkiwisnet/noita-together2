// First-party Windows/x86 G0 bridge. No executable patching, hooks, tokens or arbitrary paths.
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <sddl.h>
#include <bcrypt.h>
#include <stdint.h>
#include <string>
#include <vector>
#include <cstring>
#pragma comment(lib,"advapi32.lib")
#pragma comment(lib,"bcrypt.lib")
#ifndef NT2_PIPE_PREFIX
#define NT2_PIPE_PREFIX L"\\\\.\\pipe\\nt2-g0-v1-"
#endif
static uint8_t nonce[16];static uint32_t seed=0;static bool attached=false;
struct Liveness {HANDLE stop;uint8_t context[16];};
static HANDLE worker=nullptr;static Liveness* liveness=nullptr;
static std::wstring sid(){HANDLE token=nullptr;if(!OpenProcessToken(GetCurrentProcess(),TOKEN_QUERY,&token))return L"";DWORD n=0;GetTokenInformation(token,TokenUser,nullptr,0,&n);std::vector<uint8_t> b(n);if(!GetTokenInformation(token,TokenUser,b.data(),n,&n)){CloseHandle(token);return L"";}CloseHandle(token);LPWSTR text=nullptr;if(!ConvertSidToStringSidW(((TOKEN_USER*)b.data())->User.Sid,&text))return L"";std::wstring result(text);LocalFree(text);return result;}
static bool app(HANDLE pipe){ULONG pid=0;if(!GetNamedPipeServerProcessId(pipe,&pid))return false;HANDLE process=OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION,FALSE,pid);if(!process)return false;wchar_t image[1024];DWORD n=1024;bool ok=QueryFullProcessImageNameW(process,0,image,&n)!=0;CloseHandle(process);return ok&&_wcsicmp(image,NT2_APP_PATH)==0;}
static bool transfer(HANDLE h,bool write,void* bytes,DWORD size){OVERLAPPED o={};o.hEvent=CreateEventW(nullptr,TRUE,FALSE,nullptr);if(!o.hEvent)return false;DWORD n=0;BOOL ok=write?WriteFile(h,bytes,size,&n,&o):ReadFile(h,bytes,size,&n,&o);if(!ok&&GetLastError()==ERROR_IO_PENDING){if(WaitForSingleObject(o.hEvent,80)==WAIT_OBJECT_0)ok=GetOverlappedResult(h,&o,&n,FALSE);else{CancelIoEx(h,&o);GetOverlappedResult(h,&o,&n,TRUE);ok=FALSE;}}CloseHandle(o.hEvent);return ok&&n==size;}
static int exchange_context(uint32_t op,const uint8_t context[16],uint32_t* result=nullptr){std::wstring user=sid();if(user.empty())return 0;std::wstring path=NT2_PIPE_PREFIX+user;HANDLE pipe=CreateFileW(path.c_str(),GENERIC_READ|GENERIC_WRITE,0,nullptr,OPEN_EXISTING,FILE_FLAG_OVERLAPPED,nullptr);if(pipe==INVALID_HANDLE_VALUE&&GetLastError()==ERROR_PIPE_BUSY&&WaitNamedPipeW(path.c_str(),80))pipe=CreateFileW(path.c_str(),GENERIC_READ|GENERIC_WRITE,0,nullptr,OPEN_EXISTING,FILE_FLAG_OVERLAPPED,nullptr);if(pipe==INVALID_HANDLE_VALUE)return 0;if(!app(pipe)){CloseHandle(pipe);return 0;}
 uint8_t request[32]={};uint32_t fields[4]={0x3247544e,1,op,0};memcpy(request,fields,16);memcpy(request+16,context,16);uint8_t reply[32]={};bool ok=transfer(pipe,true,request,32)&&transfer(pipe,false,reply,32);CloseHandle(pipe);uint32_t values[4];memcpy(values,reply,16);if(!ok||values[0]!=0x3247544e||values[1]!=1||values[2]!=1||memcmp(reply+16,context,16)!=0)return 0;if(result)*result=values[3];return 1;}
static int exchange(uint32_t op){return exchange_context(op,nonce,&seed);}
// Worker owns an immutable nonce and only performs bounded local IPC. No Lua/game APIs.
// Engine pause does not stop this lease; context replacement explicitly closes it.
static DWORD WINAPI heartbeat(void* raw){Liveness* state=static_cast<Liveness*>(raw);while(WaitForSingleObject(state->stop,5000)==WAIT_TIMEOUT){if(WaitForSingleObject(state->stop,0)!=WAIT_TIMEOUT)break;exchange_context(5,state->context);}return 0;}
static bool stop_liveness(){if(!worker)return true;SetEvent(liveness->stop);if(WaitForSingleObject(worker,400)!=WAIT_OBJECT_0)return false;CloseHandle(worker);CloseHandle(liveness->stop);delete liveness;worker=nullptr;liveness=nullptr;return true;}
static bool start_liveness(){HMODULE module=nullptr;if(!GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS|GET_MODULE_HANDLE_EX_FLAG_PIN,reinterpret_cast<LPCWSTR>(&heartbeat),&module))return false;liveness=new Liveness{};liveness->stop=CreateEventW(nullptr,TRUE,FALSE,nullptr);if(!liveness->stop){delete liveness;liveness=nullptr;return false;}memcpy(liveness->context,nonce,16);worker=CreateThread(nullptr,0,heartbeat,liveness,0,nullptr);if(!worker){CloseHandle(liveness->stop);delete liveness;liveness=nullptr;return false;}return true;}
static bool saved_run(){
#ifdef NT2_TEST_NOSAVE
return false; // Isolated test DLL only; production build never sets this flag.
#else
wchar_t appdata[1024];DWORD n=GetEnvironmentVariableW(L"APPDATA",appdata,1024);if(n==0||n>=1024)return true;std::wstring root(appdata);size_t split=root.find_last_of(L"\\/");if(split==std::wstring::npos)return true;root=root.substr(0,split)+L"\\LocalLow\\Nolla_Games_Noita\\save00\\";return GetFileAttributesW((root+L"world_state.xml").c_str())!=INVALID_FILE_ATTRIBUTES||GetFileAttributesW((root+L"player.xml").c_str())!=INVALID_FILE_ATTRIBUTES;
#endif
}
extern "C" __declspec(dllexport) int __cdecl nt2_g0_prepare(uint32_t* out){if(!out)return 0;if(!stop_liveness()){attached=false;exchange(6);return 0;}if(attached)exchange(6);attached=false;if(BCryptGenRandom(nullptr,nonce,16,BCRYPT_USE_SYSTEM_PREFERRED_RNG)!=0)return 0;if(saved_run()){exchange(2);return -2;}attached=exchange(1)!=0;if(!attached)return 0;if(!start_liveness()){exchange(6);attached=false;return 0;}*out=seed;return 1;}
extern "C" __declspec(dllexport) int __cdecl nt2_g0_seed_accepted(){return attached?exchange(3):0;}
extern "C" __declspec(dllexport) int __cdecl nt2_g0_world_loaded(){return attached?exchange(4):0;}
extern "C" __declspec(dllexport) int __cdecl nt2_g1_context_closed(){if(!stop_liveness()){exchange(6);attached=false;return 0;}if(attached)exchange(6);attached=false;return 1;}
