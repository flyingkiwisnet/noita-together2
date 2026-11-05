# Building the contributor source

The public source tree supports compiling the Windows companion and its native
modules without private build artifacts or proprietary game files. It does not
include release approval records, binaries, credentials or a configured service.
An unapproved contributor build cannot impersonate the reviewed downloadable
release. Keep using the complete released ZIP to play on the public service.

## Windows prerequisites

Install Rust with the MSVC toolchain, Visual Studio C++ Build Tools (x86/x64 MSVC
and a Windows SDK), PowerShell and Python 3. Internal builds use Rust 1.99.0;
the checked-in Cargo lockfiles define the dependencies. Install the native target:

```powershell
rustup target add i686-pc-windows-msvc
```

No game executable, Lua DLL, extracted game directory, installed mod, certificate
or live service is needed for compilation. Cargo downloads its locked dependencies
using the contributor's installed toolchain. Microsoft WebView2 is an independent
prerequisite when running a Windows desktop app; it is not copied into this tree.

## Companion

From the repository root:

```powershell
./apps/desktop/tools/check-source.ps1
./apps/desktop/tools/check-source.ps1 -Build
```

The first command checks the modern `g3_live_candidate` desktop example. The second
compiles an optimized executable under `build/source-check/target/release/examples`.
Neither launches it. The helper creates `build/source-check/unapproved-runtime.json`
with `sourceApproved=false` and `installApproved=false`, computes its exact digest,
and supplies the three compile-time `NT2_REJOIN_RUNTIME` variables. These inputs
are explicitly unapproved; existing runtime identity, digest and closure checks
still apply. The helper restores its environment changes after Cargo exits.

The source export replaces private archived compile-time review/profile includes
with neutral JSON. Their original validation and expected digests are retained,
so those old approval paths refuse use. They are not reconstructed private evidence.
A real runtime release needs separate review of the source-built native/mod closure
and an explicitly matching service approval. These helpers do not grant that trust
or modify any service configuration.

Keep `packages/contracts` generated Rust codecs, schemas, golden vectors,
catalogues and presentation source assets. The desktop build script copies the
canonical UI data into `apps/desktop/ui`; it also embeds all UI assets and icons.
The Cargo path dependency `native/local-ws` must remain at its checked-in location.

## Native modules and current mod layout

```powershell
./apps/desktop/tools/build-source-native.ps1
python ./apps/desktop/tools/stage-source-mod.py
```

The native helper discovers Visual Studio with `vswhere`, builds the shared Rust
WebSocket static library for x86, and compiles the normal broker and two wrappers.
It creates three DLLs in a fresh `build/native-source` directory. Private fault
selectors are not enabled. It does not generate or select an approval.

The staging helper uses `source-mod-layout.json` to put mod sources, generated
canonical Lua/JSON catalogues and those three DLLs under
`build/source-mod/mods/noita_together2`. The raw mod source directory also retains
older entrypoints: simply copying it does not produce the current layout. The
helper validates all inputs before staging and refuses an existing output.
It does not install anything into a game or touch saves, pairing or user state.

## Tests and notices

The standalone transport tests require no game or deployment profiles:

```powershell
cargo test --locked --manifest-path native/local-ws/Cargo.toml
```

Other retained tests are useful source references; historical desktop acceptance
tests requiring archived approvals, frozen executables, an owned database fixture
or actual Noita are not a self-contained public test suite. Follow the root build
documentation for the supported service/contracts/browser checks. Compilation and
synthetic inputs do not establish actual game behavior.

Preserve root `LICENSE`, `CREDITS.md`, `THIRD_PARTY_NOTICES.md`, the desktop and
native component notices, and the existing loot/biome notices. Original NT2
geometric icon source is in `icons/beta-icon.svg` and can be regenerated using
`tools/generate-beta-icon.py` with Pillow. Upstream Noita Together Lua/artwork
retains its original MIT notice. Game-derived portions remain subject to Nolla's
modding terms and attribution; references to `data/...` load the user's game
resources and do not distribute those resources. No project notice relicenses
proprietary Noita content. Generated Cargo dependency licence bundles are release
outputs; preserve all applicable dependency notices when distributing binaries.
