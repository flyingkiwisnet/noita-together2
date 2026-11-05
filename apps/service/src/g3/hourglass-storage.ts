import type {HourglassRecord} from '../../../../packages/domain/src/g3-hourglass.js';
type Stored=HourglassRecord&{worldTokenEncoding?:string};
function map(record:HourglassRecord,convert:(s:string)=>string):HourglassRecord {
 return {digest:record.digest,plan:{...record.plan,source:{...record.plan.source,worldToken:convert(record.plan.source.worldToken)},recipients:record.plan.recipients.map(r=>({...r,worldToken:convert(r.worldToken)}))},deliveries:record.deliveries.map(d=>({...d,recipient:{...d.recipient,worldToken:convert(d.recipient.worldToken)}}))};
}
// Storage only, never wire. Canonical base64 avoids sixfold JSON escaping of
// legitimate bounded token bytes and keeps maximum retained rows below32KiB.
export function storeHourglass(record:HourglassRecord,encoded=true):Stored {
 return encoded?{...map(record,s=>Buffer.from(s,'utf8').toString('base64')),worldTokenEncoding:'base64-utf8-v1'}:structuredClone(record);
}
export function loadHourglass(stored:Stored):HourglassRecord {
 if(!stored.worldTokenEncoding)return structuredClone(stored);
 if(stored.worldTokenEncoding!=='base64-utf8-v1')throw Error('Unknown hourglass storage format');
 return map(stored,s=>{const bytes=Buffer.from(s,'base64');if(bytes.toString('base64')!==s)throw Error('Invalid hourglass storage token');return new TextDecoder('utf-8',{fatal:true}).decode(bytes);});
}
