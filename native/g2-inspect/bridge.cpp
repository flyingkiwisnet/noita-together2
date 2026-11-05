// Candidate inspection ABI. Reuse owned pause-safe transport without changing the installed bridge.
#ifndef NT2_PIPE_PREFIX
#define NT2_PIPE_PREFIX L"\\\\.\\pipe\\nt2-g2-inspect-v1-"
#endif
#define NT2_APP_HINT_SUFFIX L"\\NoitaTogether2\\g2-next\\bridge-app.txt"
#define nt2_g2_prepare nt2_g2i_prepare
#define nt2_g2_seed_accepted nt2_g2i_seed_accepted
#define nt2_g2_world_loaded nt2_g2i_world_loaded
#define nt2_g2_status nt2_g2i_status
#define nt2_g2_close nt2_g2i_close
#include "../g2-bridge/bridge.cpp"
extern "C" __declspec(dllexport) int __cdecl nt2_g2i_report(const uint8_t* context,const uint8_t* body,uint32_t size,uint64_t sequence){
    if(!current(context)||!body||size==0||size>32768||sequence==0)return 0;
    const std::wstring user=sid();if(user.empty())return 0;
    const std::wstring path=NT2_PIPE_PREFIX+user;
    HANDLE pipe=CreateFileW(path.c_str(),GENERIC_READ|GENERIC_WRITE,0,nullptr,OPEN_EXISTING,FILE_FLAG_OVERLAPPED,nullptr);
    if(pipe==INVALID_HANDLE_VALUE&&GetLastError()==ERROR_PIPE_BUSY&&WaitNamedPipeW(path.c_str(),80))pipe=CreateFileW(path.c_str(),GENERIC_READ|GENERIC_WRITE,0,nullptr,OPEN_EXISTING,FILE_FLAG_OVERLAPPED,nullptr);
    if(pipe==INVALID_HANDLE_VALUE)return 0;if(!app(pipe)){CloseHandle(pipe);return 0;}
    // One bounded message; Rust atomically validates the entire report before acknowledging.
    std::vector<uint8_t> frame(48+size,0);const uint32_t fields[4]={0x3247544e,3,7,size};
    memcpy(frame.data(),fields,16);memcpy(frame.data()+16,&sequence,8);memcpy(frame.data()+24,context,16);memcpy(frame.data()+48,body,size);
    uint8_t reply[32]={};const bool ok=transfer(pipe,true,frame.data(),static_cast<DWORD>(frame.size()))&&transfer(pipe,false,reply,32);CloseHandle(pipe);
    uint32_t values[4];memcpy(values,reply,16);
    const bool accepted=ok&&values[0]==0x3247544e&&values[1]==3&&values[2]==1&&(values[3]==1||values[3]==2)&&memcmp(reply+16,context,16)==0;
    if(accepted&&liveness){InterlockedExchange(&liveness->status,static_cast<LONG>(values[3]));InterlockedExchange64(&liveness->updated,static_cast<LONG64>(GetTickCount64()));}
    return accepted?1:0;
}
