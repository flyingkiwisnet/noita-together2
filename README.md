# Noita Together 2

A community rewrite of [Noita Together](https://github.com/Noita-Together/noita-together): separate Noita worlds with shared Core gameplay or Race goals. Project: [flyingkiwisnet/noita-together2](https://github.com/flyingkiwisnet/noita-together2).

For players, use the approved [Windows beta download](https://nt2.flyingkiwis.net/download) and [setup help](https://nt2.flyingkiwis.net/help). A supported, separately purchased Steam/GOG copy of Noita is required. The companion manages a local mod; the current native mod does not support Workshop installation. This source repository is not a portable player package.

## Build and contribute

See [BUILD.md](docs/BUILD.md) for requirements, verified source checks and contributor builds. JavaScript workspaces use Node 22.16+ within Node 22, and npm 10 or 11. Windows desktop/native work additionally requires Rust, MSVC and WebView2. Build inputs and locally generated approvals are distinct from the reviewed public service's release approvals; a contributor build is not automatically accepted by that service.

The source includes service/web implementation, canonical contracts/domain code, the desktop companion, native bridge, mod and useful tests. This clean export intentionally excludes deployment secrets, certificates, local saves/captures, installed packages, historical approval receipts and private repository history. Some historical fixture/release tooling remains as source, but its private inputs are not provided; see the build limitations rather than bypassing verification.

Keep changes scoped and run affected checks. Include reproduction steps and release/game versions with reports. Check this project's current reporting options; do not assume a particular Issues configuration. Review diagnostics before sharing and never include credentials, tokens, private pairing files or real saves.

## Credits and licensing

NT2 reuses/adapts original Noita Together source and artwork by Soler91 and contributors. Preserve the complete MIT notice, copyright 2022 Soler91, in [LICENSE](LICENSE). New component notices do not relicense game-derived or third-party material. Read [CREDITS.md](CREDITS.md), [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) and [asset provenance](docs/ASSETS.md). Noita belongs to Nolla Games; game-derived material remains subject to its applicable non-commercial modding terms. Game executables, game DLLs, data archives and extracted game directories are not included.
