// File: src/telemetry/geo.rs
// Project: snap-coin-network / src/telemetry/
// Version: 0.1.0
// Description: maxminddb wrapper — IP to lat/lon/country/city lookup

use std::net::IpAddr;
use maxminddb::{Reader, geoip2};

pub struct GeoDb {
    reader: Option<Reader<Vec<u8>>>,
}

impl GeoDb {
    pub fn open(path: &str) -> Self {
        match Reader::open_readfile(path) {
            Ok(reader) => {
                log::info!("GeoDb loaded from {}", path);
                GeoDb { reader: Some(reader) }
            }
            Err(e) => {
                log::warn!("GeoDb not loaded ({}): geo lookups disabled", e);
                GeoDb { reader: None }
            }
        }
    }

    pub fn lookup(&self, ip: IpAddr) -> (Option<f64>, Option<f64>, Option<String>, Option<String>) {
        let reader = match &self.reader {
            Some(r) => r,
            None => return (None, None, None, None),
        };

        match reader.lookup::<geoip2::City>(ip) {
            Ok(city) => {
                let lat = city.location.as_ref().and_then(|l| l.latitude);
                let lon = city.location.as_ref().and_then(|l| l.longitude);
                let country = city.country
                    .as_ref()
                    .and_then(|c| c.names.as_ref())
                    .and_then(|n| n.get("en"))
                    .map(|s| s.to_string());
                let city_name = city.city
                    .as_ref()
                    .and_then(|c| c.names.as_ref())
                    .and_then(|n| n.get("en"))
                    .map(|s| s.to_string());
                (lat, lon, country, city_name)
            }
            Err(_) => (None, None, None, None),
        }
    }
}

// File: src/telemetry/geo.rs / snap-coin-network / 2026-03-27