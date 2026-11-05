use hmac::{Hmac, Mac};
use sha2::Sha256;
use serde::{Deserialize, Serialize};
pub const PATH:&str="/nt2/local/v1";
pub const PORT:u16=25570;
pub const MAX_PAYLOAD:usize=131072;
pub const HEADER:usize=32;
pub const MAX_MESSAGE:usize=MAX_PAYLOAD+HEADER;
pub const MAX_INFLIGHT:usize=8;
pub const HANDSHAKE_MS:u64=2000;
pub fn random<const N:usize>()->Result<[u8;N],String>{let mut b=[0;N];getrandom::fill(&mut b).map_err(|_|"Local entropy unavailable")?;Ok(b)}
pub fn hex(bytes:&[u8])->String{bytes.iter().map(|b|format!("{b:02x}")).collect()}
pub fn unhex<const N:usize>(s:&str)->Result<[u8;N],String>{
 if s.len()!=N*2||!s.bytes().all(|b|b.is_ascii_digit()||(b'a'..=b'f').contains(&b)){return Err("Invalid local descriptor encoding".into())}
 let mut b=[0;N];for(i,v)in b.iter_mut().enumerate(){*v=u8::from_str_radix(&s[i*2..i*2+2],16).map_err(|_|"Invalid local descriptor encoding")?;}Ok(b)
}
#[derive(Serialize,Deserialize)]#[serde(deny_unknown_fields)]
pub struct Descriptor{pub version:u32,pub endpoint:String,pub app:String,pub secret:String}
impl Descriptor{
 pub fn validate(&self,private:bool)->Result<(std::net::SocketAddrV4,[u8;16],[u8;32]),String>{
  if self.version!=1{return Err("Unsupported local descriptor".into())}
  let endpoint=self.endpoint.strip_prefix("ws://127.0.0.1:").and_then(|s|s.strip_suffix(PATH)).ok_or("Exact local endpoint required")?;
  let port:u16=endpoint.parse().map_err(|_|"Invalid local port")?;
  if port==0||(!private&&port!=PORT)||endpoint!=port.to_string(){return Err("Invalid local port".into())}
  Ok((std::net::SocketAddrV4::new(std::net::Ipv4Addr::LOCALHOST,port),unhex(&self.app)?,unhex(&self.secret)?))
 }
}
// HMAC transcript v1: role label || app16 || native16 || clientNonce32 || serverNonce32.
// A server proves possession before receiving the client's proof. No secret on wire.
pub fn proof(secret:&[u8;32],server:bool,app:&[u8;16],native:&[u8;16],client:&[u8;32],nonce:&[u8;32])->[u8;32]{
 let mut mac=Hmac::<Sha256>::new_from_slice(secret).expect("HMAC key size");
 mac.update(if server{b"nt2/local/v1 server\0"}else{b"nt2/local/v1 client\0"});mac.update(app);mac.update(native);mac.update(client);mac.update(nonce);mac.finalize().into_bytes().into()
}
pub fn verify(secret:&[u8;32],server:bool,app:&[u8;16],native:&[u8;16],client:&[u8;32],nonce:&[u8;32],value:&[u8])->bool{
 let mut mac=Hmac::<Sha256>::new_from_slice(secret).expect("HMAC key size");mac.update(if server{b"nt2/local/v1 server\0"}else{b"nt2/local/v1 client\0"});for b in [app.as_slice(),native.as_slice(),client.as_slice(),nonce.as_slice()]{mac.update(b)}mac.verify_slice(value).is_ok()
}
pub fn auth(kind:u8,body:&[u8])->Vec<u8>{let mut b=b"NLW1\x01\0\0\0".to_vec();b[5]=kind;b.extend_from_slice(body);b}
pub fn auth_body(b:&[u8],kind:u8,length:usize)->Result<&[u8],String>{if b.len()!=length+8||b[..5]!=*b"NLW1\x01"||b[5]!=kind||b[6..8]!=[0;2]{return Err("Invalid local authentication message".into())}Ok(&b[8..])}
pub struct Envelope<'a>{pub reply:bool,pub generation:u64,pub request:u64,pub payload:&'a[u8]}
impl<'a> Envelope<'a>{
 pub fn encode(&self)->Result<Vec<u8>,String>{
  if self.generation==0||self.request==0||self.payload.len()<32||self.payload.len()>MAX_PAYLOAD{return Err("Local envelope bounds".into())}
  let mut b=vec![0;HEADER];b[..4].copy_from_slice(b"NLM1");b[4]=if self.reply{2}else{1};b[8..16].copy_from_slice(&self.generation.to_le_bytes());b[16..24].copy_from_slice(&self.request.to_le_bytes());b[24..28].copy_from_slice(&(self.payload.len()as u32).to_le_bytes());b.extend_from_slice(self.payload);Ok(b)
 }
 pub fn decode(b:&'a[u8])->Result<Self,String>{
  if b.len()<HEADER||b.len()>MAX_MESSAGE||b[..4]!=*b"NLM1"||!matches!(b[4],1|2)||b[5..8]!=[0;3]||b[28..32]!=[0;4]{return Err("Invalid local envelope".into())}
  let e=Self{reply:b[4]==2,generation:u64::from_le_bytes(b[8..16].try_into().unwrap()),request:u64::from_le_bytes(b[16..24].try_into().unwrap()),payload:&b[HEADER..]};
  if e.generation==0||e.request==0||e.payload.len()<32||e.payload.len()>MAX_PAYLOAD||u32::from_le_bytes(b[24..28].try_into().unwrap())as usize!=e.payload.len(){return Err("Local envelope bounds".into())}Ok(e)
 }
}
pub fn ws_config()->tokio_tungstenite::tungstenite::protocol::WebSocketConfig{
 tokio_tungstenite::tungstenite::protocol::WebSocketConfig::default().max_message_size(Some(MAX_MESSAGE)).max_frame_size(Some(MAX_MESSAGE)).write_buffer_size(0).max_write_buffer_size(MAX_MESSAGE*MAX_INFLIGHT)
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn mutual_proofs_bind_roles_and_all_incarnations(){let(secret,app,native,client,nonce)=([1;32],[2;16],[3;16],[4;32],[5;32]);let p=proof(&secret,true,&app,&native,&client,&nonce);assert!(verify(&secret,true,&app,&native,&client,&nonce,&p));assert!(!verify(&secret,false,&app,&native,&client,&nonce,&p));assert!(!verify(&secret,true,&app,&native,&client,&[6;32],&p));assert!(!verify(&[6;32],true,&app,&native,&client,&nonce,&p));assert!(!verify(&secret,true,&[6;16],&native,&client,&nonce,&p));}
 #[test]fn envelope_checks_every_boundary(){let b=Envelope{reply:false,generation:1,request:2,payload:&[1;32]}.encode().unwrap();assert_eq!(Envelope::decode(&b).unwrap().request,2);for i in [0,4,5,28]{let mut bad=b.clone();bad[i]=255;assert!(Envelope::decode(&bad).is_err())}assert!(Envelope::decode(&b[..b.len()-1]).is_err());assert!(Envelope{reply:false,generation:0,request:1,payload:&[0;32]}.encode().is_err());}
}
