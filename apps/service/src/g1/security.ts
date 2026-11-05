import {createHash,randomBytes,createCipheriv,createDecipheriv,timingSafeEqual} from 'node:crypto';
import {SignJWT,jwtVerify,importPKCS8,importSPKI} from 'jose';
export class PolicyError extends Error {constructor(readonly code:string, message=code,readonly retryAfterMs=0){super(message);}}
export function demand(condition:unknown,code:string):asserts condition{if(!condition)throw new PolicyError(code);}
export const secret=()=>randomBytes(32).toString('base64url');
export const hash=(value:string)=>createHash('sha256').update(value).digest('hex');
export function fingerprint(value:unknown):string{const stable=(v:unknown):unknown=>Array.isArray(v)?v.map(stable):v&&typeof v==='object'?Object.fromEntries(Object.entries(v).sort(([a],[b])=>a.localeCompare(b)).map(([k,x])=>[k,stable(x)])):v;return hash(JSON.stringify(stable(value)));}
export function matches(a:string,b:string){const left=Buffer.from(a),right=Buffer.from(b);return left.length===right.length&&timingSafeEqual(left,right);}
export interface Keys {privateKey:string;publicKey:string;recoveryKey:string;issuer:string}
export class Security {
 private signing:ReturnType<typeof importPKCS8>;private verifying:ReturnType<typeof importSPKI>;private recovery:Buffer;
 constructor(readonly keys:Keys,readonly serviceId:string){
  this.signing=importPKCS8(keys.privateKey,'EdDSA');this.verifying=importSPKI(keys.publicKey,'EdDSA');this.recovery=Buffer.from(keys.recoveryKey,'base64url');if(this.recovery.length!==32)throw Error('Recovery key must be 256 bits');
 }
 async sign(account:string,session:string,epoch:string,kind:string,jti:string,now:number,expiresAt=Math.floor(now/1000)+900){return new SignJWT({sid:session,sessionEpoch:epoch,kind,purpose:'access'}).setProtectedHeader({alg:'EdDSA',typ:'JWT'}).setIssuer(this.keys.issuer).setAudience(this.serviceId).setSubject(account).setJti(jti).setIssuedAt(Math.floor(now/1000)).setExpirationTime(expiresAt).sign(await this.signing);}
 async verify(token:string,now:number){try{return (await jwtVerify(token,await this.verifying,{algorithms:['EdDSA'],issuer:this.keys.issuer,audience:this.serviceId,typ:'JWT',currentDate:new Date(now)})).payload;}catch{throw new PolicyError('UNAUTHENTICATED');}}
 seal(value:unknown){const iv=randomBytes(12),cipher=createCipheriv('aes-256-gcm',this.recovery,iv);cipher.setAAD(Buffer.from(this.serviceId));const data=Buffer.concat([cipher.update(JSON.stringify(value),'utf8'),cipher.final()]);return Buffer.concat([iv,cipher.getAuthTag(),data]).toString('base64url');}
 open<T>(value:string):T{const b=Buffer.from(value,'base64url'),decipher=createDecipheriv('aes-256-gcm',this.recovery,b.subarray(0,12));decipher.setAAD(Buffer.from(this.serviceId));decipher.setAuthTag(b.subarray(12,28));return JSON.parse(Buffer.concat([decipher.update(b.subarray(28)),decipher.final()]).toString('utf8')) as T;}
}
