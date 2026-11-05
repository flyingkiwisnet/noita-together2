// Separate G3 local framing prototype. No transport, authentication or game hooks.
#include <cstdint>
#include <cstring>
static void put32(std::uint8_t* out, std::uint32_t value) {
    for (unsigned i=0; i<4; ++i) out[i]=static_cast<std::uint8_t>(value>>(i*8));
}
extern "C" __declspec(dllexport) int __cdecl NT2G3EncodePose(
    std::uint8_t* out, std::uint32_t capacity, const std::uint8_t* context,
    std::uint32_t sequence, std::int32_t x, std::int32_t y,
    std::int32_t vx, std::int32_t vy, std::uint32_t facing_left) {
    if (!out || !context || capacity!=64 || !sequence || facing_left>1) return 0;
    bool nonzero=false; for (unsigned i=0;i<16;++i) nonzero=nonzero || context[i]!=0;
    if (!nonzero) return 0;
    std::memset(out,0,64); std::memcpy(out,"NTG3",4); out[4]=1; out[6]=1;
    std::memcpy(out+8,context,16); put32(out+24,sequence);
    put32(out+28,static_cast<std::uint32_t>(x)); put32(out+32,static_cast<std::uint32_t>(y));
    put32(out+36,static_cast<std::uint32_t>(vx)); put32(out+40,static_cast<std::uint32_t>(vy));
    out[44]=static_cast<std::uint8_t>(facing_left); return 64;
}
