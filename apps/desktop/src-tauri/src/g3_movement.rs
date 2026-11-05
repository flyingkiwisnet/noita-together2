//! Quantization adapter for generated canonical movement, not a second protocol.
use crate::protocol_g3::wire;
fn milli(value: f64) -> Result<String, &'static str> {
    let scaled = (value * 1000.0).round();
    if !scaled.is_finite() || scaled < i32::MIN as f64 || scaled > i32::MAX as f64 {
        return Err("Movement outside supported coordinate range");
    }
    Ok((scaled as i32).to_string())
}
pub fn frame(
    sequence: u32,
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    facing_left: bool,
) -> Result<wire::MovementFrame, &'static str> {
    if sequence == 0 {
        return Err("Movement sequence cannot wrap");
    }
    Ok(wire::MovementFrame {
        sequence: Some(sequence),
        x_milli: Some(milli(x)?),
        y_milli: Some(milli(y)?),
        vx_milli: Some(milli(vx)?),
        vy_milli: Some(milli(vy)?),
        facing_left: Some(facing_left),
    })
}
pub fn coordinate(value: &str) -> Result<f64, &'static str> {
    let number = value
        .parse::<i32>()
        .map_err(|_| "Invalid movement coordinate")?;
    if number.to_string() != value {
        return Err("Noncanonical movement coordinate");
    }
    Ok(number as f64 / 1000.0)
}
#[test]
fn generated_movement_quantization_and_invalid_numbers() {
    let f = frame(1, -1.25, 2.0, 0.0, -0.01, true).unwrap();
    assert_eq!(f.x_milli.as_deref(), Some("-1250"));
    assert_eq!(coordinate(f.x_milli.as_deref().unwrap()).unwrap(), -1.25);
    assert_eq!(f.facing_left, Some(true));
    for number in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        2147483.648,
        -2147483.649,
    ] {
        assert!(frame(1, number, 0.0, 0.0, 0.0, false).is_err());
    }
    for value in ["-0", "01", "+1", "1.0", " 1", "2147483648"] {
        assert!(coordinate(value).is_err());
    }
    assert!(frame(0, 0.0, 0.0, 0.0, 0.0, false).is_err());
    assert!(frame(u32::MAX, 0.0, 0.0, 0.0, 0.0, false).is_ok());
}
