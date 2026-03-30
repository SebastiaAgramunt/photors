use std::path::PathBuf;

/// A parsed date+time used for folder placement and filename generation.
pub struct MediaDate {
    pub year: String,  // "2024"
    pub month: String, // "02"
    pub day: String,   // "09"
    pub time: String,  // "174735" or "174735042" (digits as-is from source)
}

impl MediaDate {
    /// Destination subfolder: `YYYY/MM`
    pub fn subdir(&self) -> PathBuf {
        PathBuf::from(&self.year).join(&self.month)
    }

    /// New filename stem: `YYYY.MM.DD_<time>`
    pub fn filename_stem(&self) -> String {
        format!("{}.{}.{}_{}", self.year, self.month, self.day, self.time)
    }
}

/// Parse from EXIF DateTimeOriginal: `"YYYY:MM:DD HH:MM:SS"`
pub fn from_exif_str(s: &str) -> Option<MediaDate> {
    // strip surrounding quotes that display_value() may add
    let s = s.trim().trim_matches('"');
    let b = s.as_bytes();
    if b.len() < 10 { return None; }

    // expect "YYYY:MM:DD ..."
    if b[4] != b':' || b[7] != b':' { return None; }

    let year = &s[0..4];
    let month = &s[5..7];
    let day = &s[8..10];

    let y: u32 = year.parse().ok()?;
    let m: u32 = month.parse().ok()?;
    let d: u32 = day.parse().ok()?;
    if !(1900..=2100).contains(&y) || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }

    // time: "HH:MM:SS" starting at index 11 → strip colons
    let time = if s.len() >= 19 {
        s[11..19].replace(':', "")
    } else {
        "000000".to_string()
    };

    Some(MediaDate {
        year: year.to_string(),
        month: month.to_string(),
        day: day.to_string(),
        time,
    })
}

/// Parse from a filename stem, looking for `YYYYMMDD` followed optionally by
/// `_NNNNN+`. Example: `"PXL_20240209_174735042"` → year=2024, month=02,
/// day=09, time=174735042.
pub fn from_filename(stem: &str) -> Option<MediaDate> {
    let b = stem.as_bytes();
    let len = b.len();

    let mut i = 0;
    while i + 8 <= len {
        if b[i..i + 8].iter().all(|c| c.is_ascii_digit()) {
            let year = &stem[i..i + 4];
            let month = &stem[i + 4..i + 6];
            let day = &stem[i + 6..i + 8];

            let y: u32 = year.parse().unwrap();
            let m: u32 = month.parse().unwrap();
            let d: u32 = day.parse().unwrap();

            if (1900..=2100).contains(&y) && (1..=12).contains(&m) && (1..=31).contains(&d) {
                let after = i + 8;
                let time = if after < len && matches!(b[after], b'_' | b'-' | b'T') {
                    let ts = after + 1;
                    let digit_count = b[ts..].iter().take_while(|c| c.is_ascii_digit()).count();
                    if digit_count > 0 {
                        stem[ts..ts + digit_count].to_string()
                    } else {
                        "000000".to_string()
                    }
                } else {
                    "000000".to_string()
                };

                return Some(MediaDate {
                    year: year.to_string(),
                    month: month.to_string(),
                    day: day.to_string(),
                    time,
                });
            }
        }
        i += 1;
    }
    None
}
