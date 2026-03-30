use anyhow::Result;
use exif::{In, Reader, Tag};
use std::{fs::File, io::BufReader, path::Path};

/// Metadata extracted from a media file's EXIF data.
#[derive(Debug, Default)]
pub struct MediaInfo {
    /// Date/time the photo was taken (DateTimeOriginal), e.g. "2024:03:15 14:22:01"
    pub date_taken: Option<String>,
    /// Camera manufacturer, e.g. "Apple"
    pub make: Option<String>,
    /// Camera model, e.g. "iPhone 15 Pro"
    pub model: Option<String>,
    /// GPS latitude in decimal degrees (positive = North)
    pub gps_lat: Option<f64>,
    /// GPS longitude in decimal degrees (positive = East)
    pub gps_lon: Option<f64>,
}

/// Read EXIF metadata from a file. Returns `Ok(None)` for files with no EXIF
/// (e.g. videos, PNGs without EXIF) rather than an error.
pub fn read_exif(path: &Path) -> Result<Option<MediaInfo>> {
    let file = File::open(path)?;
    let mut bufreader = BufReader::new(file);

    let exif = match Reader::new().read_from_container(&mut bufreader) {
        Ok(e) => e,
        Err(exif::Error::NotFound(_)) | Err(exif::Error::BlankValue(_)) => return Ok(None),
        Err(e) => return Err(e.into()),
    };

    let mut info = MediaInfo::default();

    if let Some(f) = exif.get_field(Tag::DateTimeOriginal, In::PRIMARY) {
        info.date_taken = Some(f.display_value().to_string());
    }
    if let Some(f) = exif.get_field(Tag::Make, In::PRIMARY) {
        info.make = Some(f.display_value().to_string());
    }
    if let Some(f) = exif.get_field(Tag::Model, In::PRIMARY) {
        info.model = Some(f.display_value().to_string());
    }

    // GPS: stored as (degrees, minutes, seconds) rationals in IFD GPS
    info.gps_lat = parse_gps_coord(
        exif.get_field(Tag::GPSLatitude, In::PRIMARY),
        exif.get_field(Tag::GPSLatitudeRef, In::PRIMARY),
    );
    info.gps_lon = parse_gps_coord(
        exif.get_field(Tag::GPSLongitude, In::PRIMARY),
        exif.get_field(Tag::GPSLongitudeRef, In::PRIMARY),
    );

    Ok(Some(info))
}

fn parse_gps_coord(
    coord_field: Option<&exif::Field>,
    ref_field: Option<&exif::Field>,
) -> Option<f64> {
    let field = coord_field?;

    // GPSLatitude/GPSLongitude are stored as three RATIONAL values: deg, min, sec
    let values = match &field.value {
        exif::Value::Rational(v) if v.len() == 3 => v,
        _ => return None,
    };

    let deg = values[0].to_f64();
    let min = values[1].to_f64();
    let sec = values[2].to_f64();
    let decimal = deg + min / 60.0 + sec / 3600.0;

    // Negate for South/West
    let reference = ref_field.map(|f| f.display_value().to_string()).unwrap_or_default();
    let sign = if reference.contains('S') || reference.contains('W') {
        -1.0
    } else {
        1.0
    };

    Some(sign * decimal)
}
