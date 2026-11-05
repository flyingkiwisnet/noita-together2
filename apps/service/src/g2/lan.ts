import {isIP} from 'node:net';
import {networkInterfaces, type NetworkInterfaceInfo} from 'node:os';
import {X509Certificate} from 'node:crypto';

export function privateIPv4(value:unknown):value is string {
 if(typeof value!=='string'||isIP(value)!==4)return false;
 const [a,b]=value.split('.').map(Number);
 return a===10||(a===172&&b!>=16&&b!<=31)||(a===192&&b===168);
}
export interface LanConfig {ip:string;port:number;origin:string;issuer:string;serviceId:string;namespace:string}
export function lanConfig(ip:unknown,port:unknown):LanConfig {
 if(!privateIPv4(ip)||!Number.isInteger(port)||(port as number)<1024||(port as number)>65535)throw Error('LAN testing requires a literal RFC1918 IPv4 address and port1024–65535');
 const tag=ip.replaceAll('.','-');const origin=`https://${ip}:${port}`;
 return {ip,port:port as number,origin,issuer:origin,serviceId:`nt2-g2-lan-${tag}`,namespace:`g2_lan_${tag.replaceAll('-','_')}_${port}`};
}
export function validateLanConfig(value:unknown):LanConfig {
 if(!value||typeof value!=='object'||Array.isArray(value))throw Error('Invalid LAN configuration');
 const v=value as Record<string,unknown>,expected=lanConfig(v.ip,v.port);
 if(Object.keys(v).sort().join()!==Object.keys(expected).sort().join()||Object.entries(expected).some(([k,x])=>v[k]!==x))throw Error('LAN origin, issuer, service identity and namespace must match the canonical private IP configuration');
 return expected;
}
export function localInterface(ip:string,interfaces:Record<string,NetworkInterfaceInfo[]|undefined>=networkInterfaces()) {
 if(!Object.values(interfaces).some(list=>list?.some(entry=>entry.family==='IPv4'&&!entry.internal&&entry.address===ip)))throw Error('Configured LAN IP is not assigned to a local private interface');
}
export function validateCertificate(cert:Buffer|string,ca:Buffer|string,ip:string,now=Date.now()) {
 const leaf=new X509Certificate(cert),root=new X509Certificate(ca);
 if(!privateIPv4(ip)||leaf.ca||!root.ca||leaf.checkIP(ip)!==ip||!leaf.verify(root.publicKey)||!root.verify(root.publicKey)||!leaf.checkIssued(root)||![leaf,root].every(c=>Date.parse(c.validFrom)<=now&&now<Date.parse(c.validTo)))throw Error('Invalid, expired or mismatched LAN certificate/IP/CA');
 if(!leaf.keyUsage?.includes('1.3.6.1.5.5.7.3.1'))throw Error('LAN certificate must allow TLS server authentication');
 return leaf;
}
