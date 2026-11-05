# Third-party notices

## Original Noita Together source and assets

Upstream: https://github.com/Noita-Together/noita-together

MIT, copyright (c) 2022 Soler91. The complete applicable notice is in the accompanying LICENSE. Preserve that notice with substantial copies/adaptations, including mod source and artwork; do not replace it with only a new NT2 copyright. Additional copies are retained in mod/noita_together2/files/loot/LICENSE and mod/noita_together2/files/g3/ui/biomes/LICENSE.

The current payload includes legacy Lua helpers and PNG/XML assets. Provenance findings and unresolved paths are tracked in docs/ASSETS.md. An upstream repository licence is not a licence grant for proprietary Noita data; no game binaries/data should be added without an explicit redistribution basis.

## New NT2 components

apps/desktop/LICENSE and native/g0-bridge/LICENSE contain MIT notices for their respective new NT2 work (copyright 2026 Noita Together 2 contributors). native/local-ws declares MIT. These component notices do not relicense legacy or third-party work. The retained upstream root LICENSE is not an exclusive authorship claim over the rewrite.

## Desktop/native dependencies

Client generates cargo-provenance.json and DEPENDENCY_LICENSES.txt in private release-verification evidence (not distributed) The portable release must include the generated Cargo dependency inventory and complete applicable licence texts for its exact locked, shipped Windows dependencies. apps/desktop/dependency-licenses.json is an earlier source inventory and does not replace the generated release bundle. Tauri, Tokio, tungstenite, reqwest, serde, UUID, SHA-2 and their transitive dependencies retain their individual notices. Microsoft WebView2 is an independently installed Microsoft runtime, obtained through https://developer.microsoft.com/en-us/microsoft-edge/webview2/; it is not relicensed by this project.

## Service/web dependencies

The service uses @fastify/websocket, @node-rs/argon2, fastify, jose, openid-client and pg, together with their locked transitive dependencies. Their package licence files remain authoritative. npm dependencies are not bundled into the browser pages or portable Windows archive merely because they appear in the source workspace. An eventual service/source export must preserve dependency notices for the exact material it distributes; package-lock.json identifies versions, not complete licence texts.

## Historical dependency

pollws: https://github.com/probable-basilisk/pollws, probable-basilisk (fakepyry/pyry), credited by original Noita Together. It is absent from the current approved payload; do not infer permission to ship old pollws binaries from this acknowledgement. If added later, include its actual licence and dependencies after review.

## Supplemental notices for cached crates without standalone texts

Missing cached files do not mean unknown licensing. The reviewed normal runtime dependency graph includes alloc-stdlib 0.3.0, selectors 0.38.0, webview2-com 0.39.1 and webview2-com-sys 0.39.1. The conservative generated inventory also includes build dependencies; defmt-parser and webview2-com-macros were not in that normal runtime graph.

### webview2-com and webview2-com-sys

MIT notice verified from upstream https://github.com/wravery/webview2-rs/blob/main/LICENSE (Bill Avery, 2021). These Rust bindings are distinct from Microsoft's separately installed WebView2 runtime.

MIT License

Copyright (c) 2021 Bill Avery

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

### alloc-stdlib

BSD-3-Clause; upstream https://github.com/dropbox/rust-alloc-no-stdlib. The repository's full Dropbox notice is reproduced below; the same text is present for alloc-no-stdlib in the generated dependency bundle.

Copyright (c) 2016 Dropbox, Inc.
All rights reserved.

Redistribution and use in source and binary forms, with or without modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following disclaimer.

2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the following disclaimer in the documentation and/or other materials provided with the distribution.

3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote products derived from this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

### selectors

selectors 0.38.0 is MPL-2.0, from https://github.com/servo/stylo. The full Mozilla Public License 2.0 text is included in DEPENDENCY_LICENSES.txt (also required for cssparser); it applies to the covered selectors source as well. Preserve its source-file notices and recipients' MPL rights. Source for this exact published crate is available from https://crates.io/api/v1/crates/selectors/0.38.0/download; the archive contains its source and Cargo metadata. When distributing covered executable code, provide the corresponding covered source by reasonable timely means, including any local modifications, under MPL-2.0. Do not substitute an unversioned repository link for the actual corresponding source. A larger combined work is not thereby wholly relicensed to MPL.

## Microsoft WebView2 SDK embedded loader

The Windows companion statically links Microsoft's x64 WebView2LoaderStatic.lib supplied by webview2-com-sys 0.39.1. This Microsoft SDK code is distinct from both Bill Avery's MIT Rust bindings and the separately installed Evergreen Runtime. The bindings crate's MIT declaration does not relicense Microsoft's loader.

The cached crate records upstream revision edc2caf886175ccaebe86078c9cfe1ae2a187328; its update-bindings source pins Microsoft.Web.WebView2 SDK 1.0.3800.47. The x64 static-loader bytes were verified identical to build/native/x64/WebView2LoaderStatic.lib in that exact official NuGet package. Evidence: private release-verification evidence (not distributed) Package source: https://api.nuget.org/v3-flatcontainer/microsoft.web.webview2/1.0.3800.47/microsoft.web.webview2.1.0.3800.47.nupkg. Microsoft's distribution guidance documents static loader distribution: https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution.

The exact package's LICENSE.txt and NOTICE.txt follow in full. Package-wide notices are retained conservatively; this does not assert every SDK tool/dependency is linked into the companion. No SDK static library, SDK tool or Evergreen Runtime installer is separately included in the portable archive merely because its notice appears here.

### Microsoft SDK LICENSE.txt

Copyright (C) Microsoft Corporation. All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are
met:

   * Redistributions of source code must retain the above copyright
notice, this list of conditions and the following disclaimer.
   * Redistributions in binary form must reproduce the above
copyright notice, this list of conditions and the following disclaimer
in the documentation and/or other materials provided with the
distribution.
   * The name of Microsoft Corporation, or the names of its contributors 
may not be used to endorse or promote products derived from this
software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
"AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
### Microsoft SDK NOTICE.txt

NOTICES AND INFORMATION
Do Not Translate or Localize

This software incorporates material from third parties. Microsoft makes certain
open source code available at https://3rdpartysource.microsoft.com, or you may
send a check or money order for US $5.00, including the product name, the open
source component name, and version number, to:

Source Code Compliance Team
Microsoft Corporation
One Microsoft Way
Redmond, WA 98052
USA

Notwithstanding any other terms, you may reverse engineer this software to the
extent required to debug changes to any libraries licensed under the GNU Lesser
General Public License.

----------------------------------------------------------------

Antlr3.Runtime 3.5.2-rc1 - BSD 3-Clause

[The "BSD license"]
Copyright (c) 2011 The ANTLR Project
All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions
are met:

 1. Redistributions of source code must retain the above copyright
    notice, this list of conditions and the following disclaimer.
 2. Redistributions in binary form must reproduce the above copyright
    notice, this list of conditions and the following disclaimer in the
    documentation and/or other materials provided with the distribution.
 3. Neither the name of the copyright holder nor the names of its
    contributors may be used to endorse or promote products derived from
    this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS OR
IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY DIRECT, INDIRECT,
INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF
THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

---------------------------------------------------------

---------------------------------------------------------

StringTemplate4 4.0.9-rc1 - BSD 3-Clause

[The "BSD license"]
Copyright (c) 2011 The ANTLR Project
All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions
are met:

 1. Redistributions of source code must retain the above copyright
    notice, this list of conditions and the following disclaimer.
 2. Redistributions in binary form must reproduce the above copyright
    notice, this list of conditions and the following disclaimer in the
    documentation and/or other materials provided with the distribution.
 3. Neither the name of the copyright holder nor the names of its
    contributors may be used to endorse or promote products derived from
    this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS OR
IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY DIRECT, INDIRECT,
INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF
THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

---------------------------------------------------------

