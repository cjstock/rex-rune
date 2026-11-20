//! The local wall clock, for telling a person when something happened.
use std::time::SystemTime;

/// `time` as the local hour and minute, `14:02`, where the system can say what local time
/// is (Unix); `None` elsewhere, so the caller leaves the time out.
pub fn local_hour_minute(time: SystemTime) -> Option<String> {
    let seconds = time.duration_since(SystemTime::UNIX_EPOCH).ok()?.as_secs();
    local(seconds as i64).map(|(hour, minute)| format!("{hour:02}:{minute:02}"))
}

#[cfg(unix)]
fn local(seconds: i64) -> Option<(i32, i32)> {
    let seconds = seconds as libc::time_t;
    // SAFETY: `tm` is plain data, and `localtime_r` writes only into it.
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    let ok = unsafe { !libc::localtime_r(&seconds, &mut tm).is_null() };
    ok.then_some((tm.tm_hour, tm.tm_min))
}

#[cfg(not(unix))]
fn local(_seconds: i64) -> Option<(i32, i32)> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_time_reads_as_hours_and_minutes_where_local_time_is_known() {
        let clock = local_hour_minute(SystemTime::now());
        assert_eq!(clock.is_some(), cfg!(unix), "Unix knows local time");
        let Some(clock) = clock else { return };
        let (hour, minute) = clock.split_once(':').unwrap();
        assert_eq!((hour.len(), minute.len()), (2, 2), "{clock}");
        assert!(hour.parse::<u8>().unwrap() < 24 && minute.parse::<u8>().unwrap() < 60);
    }
}
