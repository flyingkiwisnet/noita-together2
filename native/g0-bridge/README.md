# NT2 G0 bridge provenance

First-party Windows x86 C++20 source in `bridge.cpp`, licensed under the adjacent MIT `LICENSE`. No legacy mod, hook library, Lua proxy implementation or third-party native source is incorporated. The desktop component has its own MIT declaration; this does not assign a licence to unrelated repository content.

`build.ps1` uses Visual Studio 2022 MSVC 14.44, x86 target, `/MT /LD /W4 /WX`, linking Windows advapi32 and bcrypt. Windows kernel32 provides process/pipe/I/O APIs. The compiler statically includes its CRT; compiler/runtime distribution terms remain part of the later release inventory. No original Noita binary is redistributed.

The uniquely named output is `spikes/noita-entry/.build/g0-bridge/nt2_g0_bridge.dll`. ABI: undecorated `extern "C"`, `__cdecl`, four exports (the first three retain their G0 ABI):

- `int nt2_g0_prepare(uint32_t* out_seed)` returns 1 for preparation, 0 for no usable handoff, -2 for standard save00 Continue detection. It writes the seed only on success.
- `int nt2_g0_seed_accepted(void)` reports matching seed initialization.
- `int nt2_g0_world_loaded(void)` reports actual player spawn after seed acceptance.

- `int nt2_g1_context_closed(void)` closes ownership; returns 1 after its worker stops, or 0 if stopping could not complete within the bounded wait.

G1 adds a single immutable-context worker sending owner-bound local heartbeat operations every five seconds. It never calls game APIs. A Lua FFI lifetime guard closes ownership on collection. DLL code is pinned while worker/finalizer code can run. Stop waits up to 400 ms before refusing a fresh preparation; underlying Windows cancellation completion has no proven hard deadline. See `apps/desktop/G1-evidence.md` for isolated native/Lua checks and actual-engine limits. The generated `apps/desktop/native-bridge-profile.json` binds local build hashes and is not a signed distribution manifest.

The installed original LuaJIT x86 FFI loads this DLL. The Tauri app is x64; both peers use the same fixed 32-byte pipe protocol. Process image paths and user SID are checked on each connection. The bridge embeds the exact developer app path from the build script; relocation is unsupported by this prototype. Standard APPDATA-derived save00 file detection is fail-closed and does not modify saves.

I/O waits 80 ms per operation and optionally 80 ms for a busy pipe. Cancelled overlapped I/O is awaited before its stack buffer is released. Windows cancellation completion is not a proven hard deadline. There is no remote networking, credential payload, executable patch or main-menu hook.

Reproduce with `build.ps1`, desktop `tools/build.ps1 -Test`, and the Lua51Runner native test documented in `apps/desktop/G0-evidence.md`. Deploy only through the guarded desktop `tools/deploy-g0.ps1`; it copies the licence notice beside the DLL and verifies original lua51.dll before and after deployment. This is an unsigned G0 artifact, not a release installer.
