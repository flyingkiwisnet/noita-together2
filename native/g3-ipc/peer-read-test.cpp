// Real broker read/install code, isolated from worker/pipe/game/credential state.
#include "broker.cpp"
#include <cstdio>
#include <stdexcept>
static unsigned checks=0;
static void check(bool value,const char* message){if(!value)throw std::runtime_error(message);checks++;}
static std::vector<uint8_t> batch(){std::vector<uint8_t> value(864,0);memcpy(value.data(),"NG3R\2\1",6);memcpy(value.data()+8,context,16);uint32_t lease=200;memcpy(value.data()+24,&lease,4);return value;}
int main(){try{
 memset(context,1,16);origins=2;phase=6;status=2;health_until=GetTickCount64()+60000;uint8_t out[864];memset(out,0xcc,sizeof(out));auto fresh=batch();ULONGLONG now=GetTickCount64();const ULONGLONG health=health_until;
 check(copy(1,context,0,out,864)==0,"No batch is availability only while independently playing");
 install_peers(fresh,true,now,now);check(copy(1,context,0,out,864)==864&&out[28]==0,"Fresh valid empty roster remains an actual copied removal batch");
 peers_until=GetTickCount64();memset(out,0xcc,sizeof(out));check(copy(1,context,0,out,864)==0&&out[0]==0xcc,"Expired copy cache returns no data, not permission or denial");check(read_status(1,context)==2&&health_until==health,"Read availability cannot extend/change playing authority");
 install_peers(fresh,false,now,GetTickCount64());check(copy(1,context,0,out,864)==0,"Missing transport reply is unavailable");
 install_peers(fresh,false,now,GetTickCount64(),2);check(copy(1,context,0,out,864)==-1,"Rejected app process identity is not availability");install_peers(fresh,true,GetTickCount64(),GetTickCount64());
 auto denied=fresh;denied[5]=0;install_peers(denied,true,now,GetTickCount64());check(copy(1,context,0,out,864)==-1,"Actual denied response is fail closed");
 install_peers(fresh,false,now,GetTickCount64());check(copy(1,context,0,out,864)==-1,"Denial survives missing read");install_peers(fresh,true,now-201,now);check(copy(1,context,0,out,864)==-1,"Late valid-looking reply cannot clear denial");
 now=GetTickCount64();install_peers(fresh,true,now,now);check(copy(1,context,0,out,864)==864,"Only actual fresh valid batch clears denial");
 for(unsigned offset:{0u,4u,5u,6u,7u,8u,28u,29u}){
  auto bad=fresh;bad[offset]=offset==6?5:offset==28?33:0xff;now=GetTickCount64();install_peers(bad,true,now,now);check(copy(1,context,0,out,864)==-1,"Malformed/version/context/roster header refuses");
  install_peers(fresh,false,now,now);check(copy(1,context,0,out,864)==-1,"Malformed rejection stays latched");install_peers(fresh,true,now,now);check(copy(1,context,0,out,864)==864,"Fresh validated batch recovers malformed rejection");
 }
 auto short_reply=fresh;short_reply.resize(20);install_peers(short_reply,true,now,now);check(copy(1,context,0,out,864)==-1,"Short actual reply rejects safely");now=GetTickCount64();install_peers(fresh,true,now,now);
 uint8_t other[16];memset(other,2,16);check(copy(1,other,0,out,864)==-1,"Changed scope is never availability");check(copy(2,context,0,out,864)==-1,"Unattached origin is never availability");
 check(copy(1,context,0,nullptr,864)==-1&&copy(1,context,0,out,863)==-1,"Invalid caller buffers fail closed");
 status=1;check(copy(1,context,0,out,864)==-1,"Protection is not a gap");status=0;check(copy(1,context,0,out,864)==-1,"Revoked status is not a gap");status=2;health_until=GetTickCount64();check(copy(1,context,0,out,864)==-1,"True health expiry is not a gap");health_until=health;phase=9;check(copy(1,context,0,out,864)==-1,"Terminal scope is not a gap");
 phase=6;status=2;health_until=GetTickCount64()+60000;social_until=GetTickCount64()+1000;social[5]=1;
 uint8_t message[128]={};memcpy(message,"original bank input",19);social_input.assign(32,'s');
 check(copy(1,context,14,message,19)==1,"bank input remains independent while social input is pending");check(bank_input.size()==19&&bank_sequence==1,"bounded bank copy has its own monotonic sequence");message[0]='x';check(bank_input[0]=='o',"caller mutation cannot replace copied original");
 check(copy(1,context,14,message,19)==0&&bank_sequence==1,"pending bank original cannot be replaced");check(copy(1,context,14,message,4097)==0,"oversize bank original refused before copy");check(copy(1,other,14,message,19)==0,"foreign context cannot publish bank original");
 uint8_t bank_out[8192]={};const char* view="{\"fresh\":true}";uint32_t bank_size=static_cast<uint32_t>(strlen(view));memcpy(social+32840,&bank_size,4);memcpy(social+32848,view,bank_size);
 const ULONGLONG before_bank_read=health_until;check(copy(1,context,15,bank_out,8192)==static_cast<int>(bank_size)&&memcmp(bank_out,view,bank_size)==0,"bank footer copied exactly through production broker code");check(health_until==before_bank_read,"bank read never grants or renews authority");
 check(copy(1,other,15,bank_out,8192)==0&&copy(2,context,15,bank_out,8192)==0,"bank view enforces context and attached origin");check(copy(1,context,15,bank_out,8191)==0,"bank view requires exact bounded buffer");
 social_until=GetTickCount64();check(copy(1,context,15,bank_out,8192)==0,"expired optional view never authorizes an effect");social_until=GetTickCount64()+1000;status=1;check(copy(1,context,15,bank_out,8192)==0,"protection blocks bank view");status=2;health_until=GetTickCount64();check(copy(1,context,15,bank_out,8192)==0,"expired independent gameplay authority blocks bank");
 printf("PASS native peer/bank read %u assertions; real broker install/copy/status, no worker/pipe/game/DB\n",checks);return 0;
 }catch(const std::exception& e){fprintf(stderr,"FAIL %s\n",e.what());return 1;}}
