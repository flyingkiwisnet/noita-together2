#pragma once
#include <stdint.h>
// One process-local module owns all world state and all pipe I/O. No C++ objects
// or caller buffers cross the ABI; every exported game call copies or polls.
struct NT2IpcApi {
 uint32_t version,size;
 int (__cdecl *prepare)(uint32_t,uint32_t*,uint8_t*);
 int (__cdecl *control)(uint32_t,uint32_t,const uint8_t*,uint32_t);
 int (__cdecl *status)(uint32_t,const uint8_t*);
 int (__cdecl *report)(uint32_t,const uint8_t*,const uint8_t*,uint32_t,uint64_t);
 int (__cdecl *move)(uint32_t,const uint8_t*,const uint8_t*);
 // kind0 peers:864 valid copy,0 unavailable cache under current status2,
 // -1 invalid scope/authority or latched denied/malformed response.
 int (__cdecl *copy)(uint32_t,const uint8_t*,uint32_t,uint8_t*,uint32_t);
 int (__cdecl *diagnostic)(uint32_t,const uint8_t*);
 int (__cdecl *metrics)(uint32_t*,uint32_t);
 int (__cdecl *resume)(uint32_t,const uint8_t*,uint64_t,uint8_t*);
 int (__cdecl *startup)(uint32_t,uint8_t*,uint32_t,uint8_t*,uint32_t);
 int (__cdecl *startup_diagnostic)(uint8_t*,uint32_t);
};
using NT2IpcGetApi=const NT2IpcApi* (__cdecl *)(uint32_t);
