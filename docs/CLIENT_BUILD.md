# Windows contributor source build

Install a Rust MSVC toolchain supporting Rust edition 2024, Cargo, Visual Studio C++ Build Tools with the x86/x64 MSVC tools and Windows SDK, and WebView2 for running a desktop app. Put `cargo`/`rustup` on PATH. Python 3.9+ is needed only for the mod staging helper. Toolchains and package dependencies are not vendored.

## Desktop compilation

From a PowerShell prompt at the repository root:

```powershell
./apps/desktop/tools/check-source.ps1 -PrepareOnly
./apps/desktop/tools/check-source.ps1
# Optional optimized contributor executable:
./apps/desktop/tools/check-source.ps1 -Build
```

The helper uses installed Cargo, the modern `g3_live_candidate` example and its declared gameplay features, and generates an explicitly **unapproved** compile input under ignored `build/source-check`. `-PrepareOnly` prepares that input without compiling. It does not claim approval, package/enable a mod, launch an app/game or connect a contributor build to the reviewed public service. Matching reviewed runtime/service approval is a separate release process.

The original working tree embedded historical private profile/review receipts even in unrelated library modules. This public export replaces only those excluded `include_str!`/`include_bytes!` expressions with `{}` bytes. Their existing digest/identity/approval validation remains intact and fails closed. Historical receipt-dependent Rust acceptance tests are not claimed as public source regressions. Canonical contracts, generated Rust, schemas and presentation assets remain source inputs.

## Native and current mod layout

```powershell
rustup target add i686-pc-windows-msvc
./apps/desktop/tools/build-source-native.ps1
python apps/desktop/tools/stage-source-mod.py
```

The native helper discovers installed MSVC with `vswhere`, compiles `native/local-ws` for the x86 game, then builds the three normal broker/wrapper DLLs in a new `build/native-source` directory. It does not use a private Rust cache or copy game binaries. Compiler output is never an automatic approval. Choose fresh output directories with the helpers' options for repeated builds.

The staging helper uses `source-mod-layout.json` to map current source and canonical catalogues plus those locally built DLLs into `build/source-mod/mods/noita_together2`. The raw top-level mod `init.lua` is a retained older entry; copying the source folder directly does not produce the current runtime layout. Source-only staging creates no release manifest, trusted profile or installed mod and does not alter game files/saves.

Actual native compilation needs the stated toolchains. The source export's reference/parser checks do not prove a Windows build or actual game session. Contributor changes must obtain their own reviewed matching profile before a release; do not weaken validators or fabricate `sourceApproved=true` to force acceptance.
