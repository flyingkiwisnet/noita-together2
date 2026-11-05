//! Generated Server wire only; local association is not authorization.
pub const REVISION:&str="g3-rejoin-1";
pub const ENDPOINT:&str="/v3/rejoin/rpc";
#[allow(dead_code)]
pub mod wire {include!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../packages/contracts/g3-rejoin/g3-rejoin.rs"));}
pub fn response(bytes:&[u8])->Result<wire::Response,String>{
 if bytes.len()>65536{return Err("Resume response exceeds bound".into())}
 let response:wire::Response=serde_json::from_slice(bytes).map_err(|_|"Invalid canonical resume response")?;
 if response.major!=Some(1)||response.revision.as_deref()!=Some(REVISION){return Err("Resume contract mismatch".into())}Ok(response)
}
