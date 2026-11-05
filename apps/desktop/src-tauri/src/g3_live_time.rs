// Existing owned UTC parser; no locale or permissive TLS clock interpretation.
pub(crate) fn timestamp(s: &str) -> Result<u64, String> {
    // Canonical service timestamps are UTC ISO-8601. Avoid locale/timezone interpretation.
    let b = s.as_bytes();
    if b.len() < 20
        || b[4] != b'-'
        || b[7] != b'-'
        || b[10] != b'T'
        || b[13] != b':'
        || b[16] != b':'
        || !s.ends_with('Z')
    {
        return Err("Invalid UTC deadline".into());
    }
    let part = |a, c| {
        s.get(a..c)
            .and_then(|s| s.parse::<i64>().ok())
            .ok_or("Invalid UTC deadline")
    };
    let (y, m, d, h, min, sec) = (
        part(0, 4)?,
        part(5, 7)?,
        part(8, 10)?,
        part(11, 13)?,
        part(14, 16)?,
        part(17, 19)?,
    );
    if !(1..=12).contains(&m)
        || !(1..=31).contains(&d)
        || !(0..24).contains(&h)
        || !(0..60).contains(&min)
        || !(0..60).contains(&sec)
    {
        return Err("Invalid UTC deadline".into());
    }
    let y = y - i64::from(m <= 2);
    let era = y.div_euclid(400);
    let yo = y - era * 400;
    let mp = m + if m > 2 { -3 } else { 9 };
    let days = era * 146097 + yo * 365 + yo / 4 - yo / 100 + (153 * mp + 2) / 5 + d - 1 - 719468;
    let millis = if b.len() == 20 {
        0
    } else {
        let f = s.get(20..s.len() - 1).ok_or("Invalid UTC deadline")?;
        if b[19] != b'.' || f.len() != 3 {
            return Err("Invalid UTC deadline".into());
        }
        f.parse::<i64>().map_err(|_| "Invalid UTC deadline")?
    };
    u64::try_from((days * 86400 + h * 3600 + min * 60 + sec) * 1000 + millis)
        .map_err(|_| "Invalid UTC deadline".into())
}

