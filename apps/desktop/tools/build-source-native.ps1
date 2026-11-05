param([string]$OutputDirectory='build/native-source')
# Source-built normal x86 native ABI. No generated runtime trust or installation.
$ErrorActionPreference='Stop'
$taskRepo=Split-Path (Split-Path (Split-Path $PSScriptRoot -Parent) -Parent) -Parent
$taskOut=[IO.Path]::GetFullPath((Join-Path $taskRepo $OutputDirectory))
if(!$taskOut.StartsWith($taskRepo+[IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase) -or (Test-Path -LiteralPath $taskOut)){throw 'Choose a fresh output directory within this repository'}
$taskVsWhere=Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
if(!(Test-Path -LiteralPath $taskVsWhere)){throw 'Install Visual Studio C++ Build Tools with the x86/x64 MSVC tools and Windows SDK'}
$taskVs=& $taskVsWhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if(!$taskVs){throw 'MSVC x86/x64 C++ toolchain unavailable'}
$taskVsDev=Join-Path $taskVs 'Common7/Tools/VsDevCmd.bat'
New-Item -ItemType Directory -Path $taskOut|Out-Null
$taskTarget=Join-Path $taskOut 'rust-target'
$taskRustFlags=$env:RUSTFLAGS
try{
 $env:RUSTFLAGS='-C target-feature=+crt-static'
 & cargo build --locked --release --target i686-pc-windows-msvc --manifest-path (Join-Path $taskRepo 'native/local-ws/Cargo.toml') --target-dir $taskTarget
 if($LASTEXITCODE -ne 0){throw 'Build requires Rust MSVC toolchain and rustup target add i686-pc-windows-msvc'}
}finally{$env:RUSTFLAGS=$taskRustFlags}
$taskStatic=Join-Path $taskTarget 'i686-pc-windows-msvc/release/nt2_local_ws.lib'
$taskCommands=@('@echo off','set CL=','set _CL_=',('call "'+$taskVsDev+'" -arch=x86 -host_arch=x64 >nul'),'if errorlevel 1 exit /b 1')
foreach($taskUnit in @(@('broker.cpp','nt2_g3_ipc',''),@('wrapper.cpp','nt2_g3_transport','/DNT2_IPC_ORIGIN=1'),@('wrapper.cpp','nt2_g3_social','/DNT2_IPC_ORIGIN=2 /DNT2_IPC_SOCIAL'))){
 $taskLib=if($taskUnit[0] -eq 'broker.cpp'){' "'+$taskStatic+'" ntdll.lib userenv.lib ws2_32.lib bcrypt.lib advapi32.lib kernel32.lib synchronization.lib'}else{''}
 $taskCommands+=('cl /nologo /W4 /WX /EHsc /std:c++20 /MT /LD '+$taskUnit[2]+' "'+(Join-Path $taskRepo ('native/g3-ipc/'+$taskUnit[0]))+'" /Fo"'+$taskOut+'\'+$taskUnit[1]+'.obj" /link /Brepro /OUT:"'+$taskOut+'\'+$taskUnit[1]+'.dll" /IMPLIB:"'+$taskOut+'\'+$taskUnit[1]+'.lib"'+$taskLib)
 $taskCommands+='if errorlevel 1 exit /b 1'
}
$taskCmd=Join-Path $taskOut 'compile.cmd'
$taskCommands|Set-Content -LiteralPath $taskCmd -Encoding ascii
& cmd.exe /d /c $taskCmd
if($LASTEXITCODE -ne 0){throw 'Native source compilation failed'}
Write-Output ('Built three normal x86 DLLs in '+$taskOut+'. Not reviewed, approved, installed or packaged.')
