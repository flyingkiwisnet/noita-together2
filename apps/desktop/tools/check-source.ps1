param([switch]$Build,[switch]$PrepareOnly)
# Contributor compilation, not a runtime approval, installer or release builder.
$ErrorActionPreference='Stop'
$taskRepo=Split-Path (Split-Path (Split-Path $PSScriptRoot -Parent) -Parent) -Parent
$taskOut=Join-Path $taskRepo 'build/source-check'
New-Item -ItemType Directory -Force -Path $taskOut|Out-Null
$taskProfile=Join-Path $taskOut 'unapproved-runtime.json'
$taskJson=@{schemaVersion=1;id='nt2-source-unapproved';sourceApproved=$false;installApproved=$false;developmentOnly=$true;fixtureOnly=$false;status='Contributor compilation only; no reviewed runtime authority';requiredLocalFiles=@();nativeArtifacts=@()}|ConvertTo-Json
[IO.File]::WriteAllText($taskProfile,$taskJson+[Environment]::NewLine,[Text.UTF8Encoding]::new($false))
if($PrepareOnly){Write-Output ('Prepared unapproved compile input: '+$taskProfile);return}
$taskNames=@('NT2_REJOIN_RUNTIME','NT2_REJOIN_RUNTIME_SHA','NT2_REJOIN_RUNTIME_ID')
$taskPrior=@{};foreach($taskName in $taskNames){$taskPrior[$taskName]=[Environment]::GetEnvironmentVariable($taskName,'Process')}
try{
 $env:NT2_REJOIN_RUNTIME=$taskProfile
 $env:NT2_REJOIN_RUNTIME_SHA=(Get-FileHash -LiteralPath $taskProfile).Hash.ToLowerInvariant()
 $env:NT2_REJOIN_RUNTIME_ID='nt2-source-unapproved'
 $taskMode=if($Build){'build'}else{'check'}
 $taskArgs=@($taskMode,'--locked','--manifest-path',(Join-Path $taskRepo 'apps/desktop/src-tauri/Cargo.toml'),'--features','g3-a1-heartbeat-busy,g3-c2-live,g3-rejoin','--example','g3_live_candidate','--target-dir',(Join-Path $taskOut 'target'))
 if($Build){$taskArgs+='--release'}
 & cargo @taskArgs
 if($LASTEXITCODE -ne 0){throw 'Contributor desktop compilation failed'}
 Write-Output 'Source compilation complete. This unapproved build cannot satisfy reviewed service/runtime approval. No app or game was launched.'
}finally{foreach($taskName in $taskNames){[Environment]::SetEnvironmentVariable($taskName,$taskPrior[$taskName],'Process')}}
