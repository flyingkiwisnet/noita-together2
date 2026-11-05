// Disposable pipe/engine-fact fixture. No game, production DLL or saved data.
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <sddl.h>
#include <atomic>
#include <thread>
#include <vector>
#include <string>
#include <cstring>
#include <cstdio>
#include <stdexcept>
#include "abi.h"
#pragma comment(lib,"advapi32.lib")
static std::atomic<bool> stop{false},exhaust{false},uncertain{false};
static std::atomic<unsigned> reports{0},moves{0},socials{0},heartbeats{0};
static uint32_t permission=3000,world_policy=5;
static void check(bool ok,const char* detail){if(!ok)throw std::runtime_error(detail);}
static std::wstring user(){HANDLE t=nullptr;check(OpenProcessToken(GetCurrentProcess(),TOKEN_QUERY,&t)!=0,"token");DWORD n=0;GetTokenInformation(t,TokenUser,nullptr,0,&n);std::vector<uint8_t>b(n);check(GetTokenInformation(t,TokenUser,b.data(),n,&n)!=0,"token user");CloseHandle(t);LPWSTR s=nullptr;check(ConvertSidToStringSidW(((TOKEN_USER*)b.data())->User.Sid,&s)!=0,"sid");std::wstring r(s);LocalFree(s);return r;}
static void server(const std::wstring& path){
 while(!stop){HANDLE p=CreateNamedPipeW(path.c_str(),PIPE_ACCESS_DUPLEX,PIPE_TYPE_BYTE|PIPE_READMODE_BYTE|PIPE_WAIT,2,65536,65536,0,nullptr);if(p==INVALID_HANDLE_VALUE)return;
  if(!ConnectNamedPipe(p,nullptr)&&GetLastError()!=ERROR_PIPE_CONNECTED){CloseHandle(p);continue;}
  uint8_t first[32]={};DWORD got=0;if(!ReadFile(p,first,32,&got,nullptr)||got!=32){CloseHandle(p);continue;}
  std::vector<uint8_t> request(first,first+32),reply;
  const bool movement=memcmp(first,"NTG3",4)==0,social=memcmp(first,"NSP1",4)==0;
  uint32_t op=0;memcpy(&op,first+8,4);uint32_t size=0;
  if(movement||social){request.resize(64);ReadFile(p,request.data()+32,32,&got,nullptr);}
  else if(op==11){request.resize(40);ReadFile(p,request.data()+32,8,&got,nullptr);}
  else if(op==7){memcpy(&size,first+12,4);check(size<=32768,"report bound");request.resize(48+size);ReadFile(p,request.data()+32,16+size,&got,nullptr);reports++;}
  Sleep(40); // Actual delayed app consumption under simultaneous lane pressure.
  if(exhaust||(op==7&&uncertain)){CloseHandle(p);continue;}
  if(movement){moves++;reply.assign(864,0);memcpy(reply.data(),"NG3R\2\1",6);memcpy(reply.data()+8,first+8,16);uint32_t lease=200;memcpy(reply.data()+24,&lease,4);}
  else if(social){socials++;reply.assign(41040,0);memcpy(reply.data(),"NSR1\1\1",6);memcpy(reply.data()+8,first+8,16);uint32_t epoch=1,lease=250;memcpy(reply.data()+24,&epoch,4);memcpy(reply.data()+28,&lease,4);}
  else{reply.assign(48,0);uint32_t fields[4]={0x3247544e,op==7?5u:op==11?6u:4u,1,op==1?(world_policy==2?0u:424242u):op==10?world_policy:2u};memcpy(reply.data(),fields,16);memcpy(reply.data()+16,request.data()+(op==7?24:16),16);uint32_t lease=permission,epoch=7;memcpy(reply.data()+32,&lease,4);memcpy(reply.data()+36,&epoch,4);if(op==7)memcpy(reply.data()+40,request.data()+16,8);if(op==5)heartbeats++;}
  DWORD sent=0;WriteFile(p,reply.data(),static_cast<DWORD>(reply.size()),&sent,nullptr);FlushFileBuffers(p);DisconnectNamedPipe(p);CloseHandle(p);
 }
}
template<class F>static bool wait(F f,unsigned ms){ULONGLONG end=GetTickCount64()+ms;do{if(f())return true;Sleep(1);}while(GetTickCount64()<end);return false;}
int wmain(int argc,wchar_t** argv){
 if(argc!=3&&argc!=4)return 2;if(argc==4&&wcscmp(argv[3],L"late-lease")==0)permission=10;if(argc==4&&(wcscmp(argv[3],L"individual")==0||wcscmp(argv[3],L"individual-wide")==0))world_policy=2;if(argc==4&&wcscmp(argv[3],L"policy-missing")==0)world_policy=0;const std::wstring path=std::wstring(argv[2])+user();std::thread consumer(server,path),standby(server,path);int result=0;
 try{
  HMODULE primary=LoadLibraryW((std::wstring(argv[1])+L"\\nt2_g3_transport.dll").c_str()),social=LoadLibraryW((std::wstring(argv[1])+L"\\nt2_g3_social.dll").c_str());check(primary&&social,"load private wrappers");
  auto get1=reinterpret_cast<const NT2IpcApi*(__cdecl*)()>(GetProcAddress(primary,"nt2_g3_private_api"));auto get2=reinterpret_cast<const NT2IpcApi*(__cdecl*)()>(GetProcAddress(social,"nt2_g3_private_api"));check(get1&&get2,"private ABI exports");auto a=get1();check(a&&a==get2()&&a->version==1&&a->size==sizeof(NT2IpcApi),"two images share exact broker/API");
  auto startup=reinterpret_cast<int(__cdecl*)(uint32_t*,uint8_t*)>(GetProcAddress(primary,"nt2_g3_startup_prepare"));check(startup!=nullptr,"startup export");uint8_t context[16]={},other[16]={};uint32_t seed=0;ULONGLONG at=GetTickCount64();int prepared=startup(&seed,context);
  if(argc==4&&wcscmp(argv[3],L"individual")!=0&&wcscmp(argv[3],L"individual-wide")!=0){int expected=wcscmp(argv[3],L"continue")==0?-2:0;check(prepared==expected&&seed==0&&a->status(1,context)==0&&GetTickCount64()-at<250,"startup guard/deadline/Continue refuses seed and authority");check(reports==0&&moves==0&&socials==0,"refusal cannot enter optional lanes");printf("PASS native startup refusal %ls; no seed/authority/game lanes\n",argv[3]);}
  else{
  check(prepared==1&&seed==(world_policy==2?0u:424242u)&&GetTickCount64()-at<250,"pre-generation seed within original startup budget");check(a->prepare(2,&seed,other)==1&&!memcmp(context,other,16),"social joins primary exact world");
  auto policy1=reinterpret_cast<uint32_t(__cdecl*)(const uint8_t*)>(GetProcAddress(primary,"nt2_g3_world_policy"));auto policy2=reinterpret_cast<uint32_t(__cdecl*)(const uint8_t*)>(GetProcAddress(social,"nt2_g3_world_policy"));check(policy1&&policy2&&policy1(context)==world_policy&&policy2(other)==world_policy,"primary/social exact cached policy");uint8_t foreign[16]={};check(policy1(foreign)==0,"policy wrong context refuses");
  uint32_t actual=world_policy==2?777u:seed;
  auto known=reinterpret_cast<int(__cdecl*)(const uint8_t*,const uint8_t*,uint32_t)>(GetProcAddress(primary,"nt2_g3_seed_known"));check(known!=nullptr,"known uint64 seed export");const char* digits=world_policy==2?"18446744073709551615":"424242";
  check(known(context,reinterpret_cast<const uint8_t*>("01"),2)==0&&known(context,reinterpret_cast<const uint8_t*>("18446744073709551616"),20)==0,"noncanonical/overflow actual seed refuses");
  check(wait([&]{return known(context,reinterpret_cast<const uint8_t*>(digits),static_cast<uint32_t>(strlen(digits)))==1;},300),"asynchronous seed acknowledgment");check(a->control(2,3,context,actual+1)==0,"different actual seed cannot replace accepted world");
  check(wait([&]{return a->control(2,4,context,1)==1;},300),"asynchronous loaded acknowledgment");
  const uint8_t body[]={'{','}'};check(wait([&]{return a->report(1,context,body,2,1)==1;},300),"immutable inspection report acknowledgment");check(reports==1,"report polls do not replay writes");check(a->status(1,context)==2&&a->status(2,other)==2,"shared playing/contact status");
  uint8_t frame[64]={};memcpy(frame,"NTG3\1\0\1\0",8);memcpy(frame+8,context,16);unsigned sequence=0,max_us=0;LARGE_INTEGER frequency;QueryPerformanceFrequency(&frequency);at=GetTickCount64();
  while(GetTickCount64()-at<4500){for(unsigned k=0;k<20;k++){sequence++;memcpy(frame+24,&sequence,4);LARGE_INTEGER begin,end;QueryPerformanceCounter(&begin);check(a->move(k%2+1,context,frame)==1,"timely verified movement remains accepted");QueryPerformanceCounter(&end);unsigned us=static_cast<unsigned>((end.QuadPart-begin.QuadPart)*1000000/frequency.QuadPart);if(us>max_us)max_us=us;}Sleep(5);check(a->status(1,context)==2,"no false contact hold under delayed consumer pressure");}
  uint32_t m1[9]={},m2[9]={};check(a->metrics(m1,9)==1&&get2()->metrics(m2,9)==1&&m1[0]!=0&&m1[0]==m2[0],"one actual worker");check(m1[5]>100&&moves>20&&socials>20&&heartbeats>=2,"coalescing and control/social/movement fairness");check(m1[7]<=131072&&m1[8]<=131072,"bounded memory");check(max_us<10000,"recurring exports never wait for pipe consumer");
  other[0]^=1;check(a->status(2,other)==0&&a->move(2,other,frame)==0,"wrong context refusal");
  uncertain=true;unsigned prior=reports;check(a->report(1,context,body,2,2)==0,"new report queued");check(wait([&]{return reports>prior;},300),"uncertain report reached pipe");Sleep(120);for(int i=0;i<50;i++)check(a->report(2,context,body,2,2)==0,"uncertain original stays uncertain");check(reports==prior+1,"no post-write report replay");
  check(wait([&]{return a->status(1,context)==2&&a->status(2,context)==2;},2500),"only verified heartbeat restores original health after report uncertainty");
  exhaust=true;at=GetTickCount64();check(wait([&]{return a->status(1,context)==0&&a->status(2,context)==0;},4000),"genuine contact exhaustion protects both wrappers");check(GetTickCount64()-at>=2000,"exhaustion observes original permission deadline");
  printf("PASS shared broker worker=%u moves=%u social=%u heartbeat=%u coalesced=%u memory_max=%u export_max_us=%u; immutable originals, startup seed, delayed consumer, fairness, wrong context and exhaustion\n",m1[0],moves.load(),socials.load(),heartbeats.load(),m1[5],m1[8],max_us);
  check(a->control(1,6,context,0)==1,"retire exact world");check(a->control(2,3,context,seed)==0&&a->move(1,context,frame)==0,"late retired control and pose refusal");
  }
 }catch(const std::exception& e){fprintf(stderr,"FAIL %s\n",e.what());result=1;}
 stop=true;for(int i=0;i<2;i++){HANDLE poke=CreateFileW(path.c_str(),GENERIC_READ|GENERIC_WRITE,0,nullptr,OPEN_EXISTING,0,nullptr);if(poke!=INVALID_HANDLE_VALUE)CloseHandle(poke);Sleep(10);}consumer.join();standby.join();return result;
}
