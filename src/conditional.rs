// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) 2023 - 2026 HTTP Handle

//! Conditional GET/HEAD (RFC 9110 §13): `If-None-Match` and
//! `If-Modified-Since` against a file's validators, and the
//! `Last-Modified` header in IMF-fixdate form.

use crate::request::Request;
use crate::response::Response;
use std::time::{SystemTime, UNIX_EPOCH};

const DAYS: [&str; 7] =
    ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep",
    "Oct", "Nov", "Dec",
];

/// Whether the client's cached copy is current, so a `304 Not Modified`
/// may be sent.
///
/// `If-None-Match` takes precedence: when present, `If-Modified-Since`
/// is ignored (RFC 9110 §13.1.3). Entity tags use the weak comparison,
/// so `W/"x"` and `"x"` match, and `*` matches any existing file. An
/// `If-Modified-Since` date that does not parse is ignored.
pub(crate) fn is_not_modified(
    request: &Request,
    etag: &str,
    modified: Option<SystemTime>,
) -> bool {
    if let Some(candidates) = request.header("if-none-match") {
        return etag_list_matches(candidates, etag);
    }
    let since = request
        .header("if-modified-since")
        .and_then(parse_http_date);
    match (since, modified.and_then(unix_secs)) {
        (Some(since), Some(modified)) => modified <= since,
        _ => false,
    }
}

fn etag_list_matches(candidates: &str, etag: &str) -> bool {
    let opaque =
        |tag: &str| tag.trim().trim_start_matches("W/").to_owned();
    let current = opaque(etag);
    candidates
        .split(',')
        .any(|tag| tag.trim() == "*" || opaque(tag) == current)
}

fn unix_secs(time: SystemTime) -> Option<u64> {
    time.duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs())
}

/// Formats `time` as an IMF-fixdate, for example
/// `Sun, 06 Nov 1994 08:49:37 GMT`. Times before 1970 return `None`.
pub(crate) fn format_http_date(time: SystemTime) -> Option<String> {
    let secs = unix_secs(time)?;
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (year, month, day) = civil_from_days(days);
    Some(format!(
        "{}, {:02} {} {} {:02}:{:02}:{:02} GMT",
        DAYS[(days % 7) as usize],
        day,
        MONTHS[(month - 1) as usize],
        year,
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    ))
}

/// Parses an IMF-fixdate into seconds since the Unix epoch.
pub(crate) fn parse_http_date(value: &str) -> Option<u64> {
    let parts: Vec<&str> = value.split_ascii_whitespace().collect();
    let [weekday, day, month, year, clock, "GMT"] = parts.as_slice()
    else {
        return None;
    };
    if !weekday.ends_with(',') {
        return None;
    }
    Some(parse_date(day, month, year)? * 86_400 + parse_clock(clock)?)
}

/// Days since the epoch for `29`, `Feb`, `2024`.
fn parse_date(day: &str, month: &str, year: &str) -> Option<u64> {
    let day = day.parse().ok().filter(|d| (1..=31).contains(d))?;
    let month = MONTHS.iter().position(|m| *m == month)? as u64 + 1;
    let year = year.parse().ok().filter(|y| *y >= 1970)?;
    Some(days_from_civil(year, month, day))
}

/// Seconds into the day for `12:34:56`.
fn parse_clock(clock: &str) -> Option<u64> {
    let fields: Vec<u64> = clock
        .split(':')
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    let [h, m, s] = fields[..] else {
        return None;
    };
    (h < 24 && m < 60 && s < 61).then_some(h * 3600 + m * 60 + s)
}

/// A `304 Not Modified` carrying the validators (and `Vary`) the full
/// response would have.
pub(crate) fn not_modified_response(
    etag: &str,
    last_modified: Option<&str>,
    varies_by_encoding: bool,
) -> Response {
    let mut response = Response::new(304, "NOT MODIFIED", Vec::new());
    response.add_header("ETag", etag);
    if let Some(date) = last_modified {
        response.add_header("Last-Modified", date);
    }
    if varies_by_encoding {
        response.add_header("Vary", "Accept-Encoding");
    }
    response
}

/// `SystemTime` for whole seconds since the epoch.
#[cfg(test)]
pub(crate) fn from_unix_secs(secs: u64) -> SystemTime {
    UNIX_EPOCH + std::time::Duration::from_secs(secs)
}

// Howard Hinnant's civil-date algorithms, restricted to dates from
// 1970 on so every quantity stays unsigned.
fn days_from_civil(year: u64, month: u64, day: u64) -> u64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year / 400;
    let yoe = year - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(days: u64) -> (u64, u64, u64) {
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + u64::from(month <= 2);
    (year, month, day)
}
