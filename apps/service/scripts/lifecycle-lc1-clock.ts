import {performance} from 'node:perf_hooks';
// Explicit-time tests remain the default. Ordinary network tests can opt into
// elapsed time without depending on how often their observer polls stdin.
export class LifecycleFixtureClock {
 private mode:'manual'|'elapsed'='manual';private base:number;private anchor:number;private last:number;private sequence=0;private revision=0;
 private injected=0;private setup=0;private ignoredWall=0;
 constructor(initial=Date.now(),private monotonic:()=>number=()=>performance.now()){this.base=initial;this.last=initial;this.anchor=monotonic();}
 now=()=>{const value=Math.floor(this.base+(this.mode==='elapsed'?Math.max(0,this.monotonic()-this.anchor):0));this.last=Math.max(this.last,value);return this.last;};
 setMode(mode:'manual'|'elapsed'){
  if(!['manual','elapsed'].includes(mode))throw Error('FIXTURE_CLOCK_ARGUMENT');
  if(this.mode==='elapsed'&&mode==='manual')throw Error('FIXTURE_CLOCK_MODE');
  if(mode!==this.mode){this.base=this.now();this.anchor=this.monotonic();this.mode=mode;this.revision++;}return this.status();
 }
 advance(ms:number,kind?:'wall'|'injected'){
  if(!Number.isInteger(ms)||ms<0||ms>3600000||kind!==undefined&&!['wall','injected'].includes(kind)||this.mode==='elapsed'&&kind===undefined)throw Error('FIXTURE_CLOCK_ARGUMENT');
  if(this.mode==='elapsed'&&kind==='wall'){this.ignoredWall+=ms;return this.status();}
  this.base+=ms;this.injected+=ms;return this.status();
 }
 rpcStep(){if(this.mode==='manual'){this.base+=120;this.setup+=120;}}
 status(){const current=this.now();return {revision:1,mode:this.mode,modeRevision:this.revision,sampleSequence:++this.sequence,elapsedSinceAnchorMs:this.mode==='elapsed'?Math.max(0,Math.floor(this.monotonic()-this.anchor)):0,explicitInjectedMs:this.injected,setupAdvanceMs:this.setup,ignoredWallAdvanceMs:this.ignoredWall,monotonicSampleMs:current,elapsedRequiresAdvanceKind:this.mode==='elapsed',ordinaryWallAdvance:'kind=wall; ignored in elapsed mode',expiryAdvance:'kind=injected; adds offset in either mode'};}
}
