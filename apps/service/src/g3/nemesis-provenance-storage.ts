import type {NemesisKillOriginal,NemesisShopOriginal} from '../../../../packages/domain/src/g3-nemesis-provenance.js';
type Original=NemesisKillOriginal|NemesisShopOriginal;
export function storeNemesisProvenance<T extends Original>(p:T){return {...structuredClone(p),worldTokenEncoding:'base64-utf8-v1',source:{...p.source,worldToken:Buffer.from(p.source.worldToken).toString('base64')}};}
export function loadNemesisProvenance<T extends Original>(p:ReturnType<typeof storeNemesisProvenance<T>>):T {
 if(p.worldTokenEncoding!=='base64-utf8-v1')throw Error('Invalid Nemesis provenance storage');const bytes=Buffer.from(p.source.worldToken,'base64');if(bytes.toString('base64')!==p.source.worldToken)throw Error('Invalid Nemesis provenance token');
 const {worldTokenEncoding:_encoding,...body}=structuredClone(p);return {...body,source:{...body.source,worldToken:new TextDecoder('utf-8',{fatal:true}).decode(bytes)}} as T;
}
