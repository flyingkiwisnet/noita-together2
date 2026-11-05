import {randomBytes,scrypt as derive,timingSafeEqual} from 'node:crypto';
import {promisify} from 'node:util';
const scrypt=promisify(derive);
export async function passwordVerifier(password:string){
 const salt=randomBytes(16).toString('hex');const digest=await scrypt(password,salt,32) as Buffer;return salt+':'+digest.toString('hex');
}
export async function passwordMatches(password:string,verifier:string){
 const [salt,digest]=verifier.split(':');if(!salt||!digest||!/^[a-f0-9]{64}$/.test(digest))return false;
 return timingSafeEqual(await scrypt(password,salt,32) as Buffer,Buffer.from(digest,'hex'));
}
