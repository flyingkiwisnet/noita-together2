//! Local bounded framing prototype, consumed only behind a future authenticated owner binding.
use crate::protocol_g3::wire;
pub const BYTES: usize = 64;
/// Caller obtains this context from its verified local owner, never a network peer.
/// A context match does not itself establish PID, SID, pipe or service authentication.
pub fn decode(
    bytes: &[u8],
    expected_context: &[u8; 16],
    last_sequence: u32,
) -> Result<wire::MovementFrame, &'static str> {
    if bytes.len() != BYTES
        || &bytes[..4] != b"NTG3"
        || bytes[4..8] != [1, 0, 1, 0]
        || bytes[8..24] != *expected_context
        || expected_context.iter().all(|b| *b == 0)
        || bytes[44] > 1
        || bytes[45..].iter().any(|b| *b != 0)
    {
        return Err("Invalid local movement frame");
    }
    let read = |offset| u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
    let sequence = read(24);
    if sequence == 0 || sequence <= last_sequence {
        return Err("Stale or wrapped local movement sequence");
    }
    Ok(wire::MovementFrame {
        sequence: Some(sequence),
        x_milli: Some((read(28) as i32).to_string()),
        y_milli: Some((read(32) as i32).to_string()),
        vx_milli: Some((read(36) as i32).to_string()),
        vy_milli: Some((read(40) as i32).to_string()),
        facing_left: Some(bytes[44] == 1),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> [u8; 64] {
        let mut bytes = [0; 64];
        bytes[..8].copy_from_slice(&[78, 84, 71, 51, 1, 0, 1, 0]);
        bytes[8..24].fill(1);
        for (offset, value) in [(24, 1i32), (28, -1250), (32, 2000), (36, 0), (40, -10)] {
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        bytes[44] = 1;
        bytes
    }
    #[test]
    fn framing_refuses_wrong_context_replay_wrap_and_reserved_bytes() {
        let bytes = fixture();
        let pose = decode(&bytes, &[1; 16], 0).unwrap();
        assert_eq!(pose.x_milli.as_deref(), Some("-1250"));
        assert!(decode(&bytes, &[2; 16], 0).is_err());
        assert!(decode(&bytes, &[1; 16], 1).is_err());
        assert!(decode(&bytes, &[1; 16], u32::MAX).is_err());
        assert!(decode(&bytes[..63], &[1; 16], 0).is_err());
        for offset in [0, 4, 5, 6, 7, 44, 45, 63] {
            let mut corrupt = bytes;
            corrupt[offset] = 255;
            assert!(decode(&corrupt, &[1; 16], 0).is_err());
        }
    }
    #[test]
    #[ignore = "Run only with the separately built x86 LuaJIT/native encoder fixture"]
    fn actual_x86_encoder_matches_rust_decoder() {
        let path = std::env::var_os("NT2_G3_NATIVE_FRAME").expect("explicit native fixture path");
        let bytes = std::fs::read(path).unwrap();
        assert_eq!(bytes, fixture());
        assert_eq!(
            decode(&bytes, &[1; 16], 0).unwrap().vy_milli.as_deref(),
            Some("-10")
        );
    }
}
