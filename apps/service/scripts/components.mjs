import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';
const root=fileURLToPath(new URL('../../../',import.meta.url));
function run(executable,args) {
  console.log(`Component command: ${[executable,...args].join(' ')}`);
  const result=spawnSync(executable,args,{cwd:root,stdio:'inherit',shell:false});
  if (result.error) throw result.error;
  if (result.status!==0) process.exit(result.status ?? 1);
}
function npm(name) {
  if (!process.env.npm_execpath) throw new Error('Run component commands through npm scripts');
  run(process.execPath,[process.env.npm_execpath,'run',name]);
}
const windowsCommands={
  'test:client':['apps/desktop/tools/build.ps1','-Test'],
  'build:native':['native/g0-bridge/build.ps1'],
  'build:client':['apps/desktop/tools/build.ps1']
};
// A reviewable plan also lets CI validate registration without launching builds or games.
export function componentPlan(action, platform=process.platform) {
  const windows=platform==='win32';
  if (Object.hasOwn(windowsCommands,action)) {
    if (!windows) throw new Error(`${action} requires the declared Windows/MSVC/Rust environment`);
    return [{kind:'powershell',args:windowsCommands[action]}];
  }
  if (action==='check') return [
    ...['contracts:check','check:server','check:web'].map(name=>({kind:'npm',name})),
    {kind:'node',args:['apps/desktop/tools/check-boundary.mjs']}
  ];
  if (action!=='test' && action!=='build') throw new Error('Unknown component command');
  const plan=['contracts:check',`${action}:server`,`${action}:web`].map(name=>({kind:'npm',name}));
  if (windows) for (const name of action==='test' ? ['test:client'] : ['build:native','build:client']) plan.push({kind:'npm',name});
  else plan.push({kind:'skipped',reason:'Windows desktop/native checks not run on this platform; service/web-only evidence.'});
  return plan;
}
if (process.argv[1] && resolve(process.argv[1])===fileURLToPath(import.meta.url)) {
  const plan=componentPlan(process.argv[2]);
  if (process.argv.includes('--plan')) console.log(JSON.stringify(plan,null,2));
  else for (const step of plan) {
    if (step.kind==='npm') npm(step.name);
    else if (step.kind==='node') run(process.execPath,step.args);
    else if (step.kind==='powershell') run('powershell.exe',['-NoProfile','-ExecutionPolicy','Bypass','-File',...step.args]);
    else console.log(step.reason);
  }
}
