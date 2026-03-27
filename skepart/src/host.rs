use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{RtError, RtErrorKind, RtHandle, RtHandleKind, RtResult, RtString};

pub enum RtNetResource {
    Placeholder(RtHandleKind),
    TcpStream(TcpStream),
    TcpListener(TcpListener),
}

impl RtNetResource {
    pub fn kind(&self) -> RtHandleKind {
        match self {
            Self::Placeholder(kind) => *kind,
            Self::TcpStream(_) => RtHandleKind::Socket,
            Self::TcpListener(_) => RtHandleKind::Listener,
        }
    }
}

#[derive(Default)]
pub struct RtNetResourceTable {
    next_handle_id: usize,
    resources: HashMap<usize, RtNetResource>,
}

impl RtNetResourceTable {
    pub fn alloc_placeholder(&mut self, kind: RtHandleKind) -> RtHandle {
        self.insert(RtNetResource::Placeholder(kind))
    }

    pub fn insert_socket(&mut self, stream: TcpStream) -> RtHandle {
        self.insert(RtNetResource::TcpStream(stream))
    }

    pub fn insert_listener(&mut self, listener: TcpListener) -> RtHandle {
        self.insert(RtNetResource::TcpListener(listener))
    }

    pub fn kind_of(&self, handle: RtHandle) -> RtResult<RtHandleKind> {
        let actual = self
            .resources
            .get(&handle.id)
            .map(RtNetResource::kind)
            .ok_or_else(|| RtError::invalid_handle(format!("unknown handle id {}", handle.id)))?;
        if actual != handle.kind {
            return Err(RtError::invalid_handle_kind(
                handle.kind.type_name(),
                actual.type_name(),
            ));
        }
        Ok(actual)
    }

    pub fn socket_mut(&mut self, handle: RtHandle) -> RtResult<&mut TcpStream> {
        self.kind_of(handle)?;
        match self.resources.get_mut(&handle.id) {
            Some(RtNetResource::TcpStream(stream)) => Ok(stream),
            Some(other) => Err(RtError::invalid_handle_kind(
                RtHandleKind::Socket.type_name(),
                other.kind().type_name(),
            )),
            None => Err(RtError::invalid_handle(format!(
                "unknown handle id {}",
                handle.id
            ))),
        }
    }

    pub fn listener_mut(&mut self, handle: RtHandle) -> RtResult<&mut TcpListener> {
        self.kind_of(handle)?;
        match self.resources.get_mut(&handle.id) {
            Some(RtNetResource::TcpListener(listener)) => Ok(listener),
            Some(other) => Err(RtError::invalid_handle_kind(
                RtHandleKind::Listener.type_name(),
                other.kind().type_name(),
            )),
            None => Err(RtError::invalid_handle(format!(
                "unknown handle id {}",
                handle.id
            ))),
        }
    }

    pub fn remove(&mut self, handle: RtHandle) -> RtResult<RtNetResource> {
        self.kind_of(handle)?;
        self.resources
            .remove(&handle.id)
            .ok_or_else(|| RtError::invalid_handle(format!("unknown handle id {}", handle.id)))
    }

    fn insert(&mut self, resource: RtNetResource) -> RtHandle {
        let handle = RtHandle {
            id: self.next_handle_id,
            kind: resource.kind(),
        };
        self.next_handle_id += 1;
        self.resources.insert(handle.id, resource);
        handle
    }
}

pub trait RtHost {
    fn io_print(&mut self, text: &str) -> RtResult<()>;

    fn io_println(&mut self, text: &str) -> RtResult<()> {
        self.io_print(text)?;
        self.io_print("\n")
    }

    fn io_read_line(&mut self) -> RtResult<RtString> {
        Ok(RtString::from(""))
    }

    fn datetime_now_unix(&mut self) -> RtResult<i64> {
        Err(RtError::unsupported_builtin("datetime.nowUnix"))
    }

    fn datetime_now_millis(&mut self) -> RtResult<i64> {
        Err(RtError::unsupported_builtin("datetime.nowMillis"))
    }

    fn datetime_from_unix(&mut self, _value: i64) -> RtResult<RtString> {
        Err(RtError::unsupported_builtin("datetime.fromUnix"))
    }

    fn datetime_from_millis(&mut self, _value: i64) -> RtResult<RtString> {
        Err(RtError::unsupported_builtin("datetime.fromMillis"))
    }

    fn datetime_parse_unix(&mut self, _value: &str) -> RtResult<i64> {
        Err(RtError::unsupported_builtin("datetime.parseUnix"))
    }

    fn datetime_component(&mut self, _name: &str, _value: i64) -> RtResult<i64> {
        Err(RtError::unsupported_builtin("datetime.component"))
    }

    fn random_seed(&mut self, _seed: i64) -> RtResult<()> {
        Err(RtError::unsupported_builtin("random.seed"))
    }

    fn random_int(&mut self, _min: i64, _max: i64) -> RtResult<i64> {
        Err(RtError::unsupported_builtin("random.int"))
    }

    fn random_float(&mut self) -> RtResult<f64> {
        Err(RtError::unsupported_builtin("random.float"))
    }

    fn fs_exists(&mut self, _path: &str) -> RtResult<bool> {
        Err(RtError::unsupported_builtin("fs.exists"))
    }

    fn fs_read_text(&mut self, _path: &str) -> RtResult<RtString> {
        Err(RtError::unsupported_builtin("fs.readText"))
    }

    fn fs_write_text(&mut self, _path: &str, _text: &str) -> RtResult<()> {
        Err(RtError::unsupported_builtin("fs.writeText"))
    }

    fn fs_append_text(&mut self, _path: &str, _text: &str) -> RtResult<()> {
        Err(RtError::unsupported_builtin("fs.appendText"))
    }

    fn fs_mkdir_all(&mut self, _path: &str) -> RtResult<()> {
        Err(RtError::unsupported_builtin("fs.mkdirAll"))
    }

    fn fs_remove_file(&mut self, _path: &str) -> RtResult<()> {
        Err(RtError::unsupported_builtin("fs.removeFile"))
    }

    fn fs_remove_dir_all(&mut self, _path: &str) -> RtResult<()> {
        Err(RtError::unsupported_builtin("fs.removeDirAll"))
    }

    fn fs_join(&mut self, _left: &str, _right: &str) -> RtResult<RtString> {
        Err(RtError::unsupported_builtin("fs.join"))
    }

    fn os_platform(&mut self) -> RtResult<RtString> {
        Err(RtError::unsupported_builtin("os.platform"))
    }

    fn os_arch(&mut self) -> RtResult<RtString> {
        Err(RtError::unsupported_builtin("os.arch"))
    }

    fn os_arg(&mut self, _index: i64) -> RtResult<RtString> {
        Err(RtError::unsupported_builtin("os.arg"))
    }

    fn os_env_has(&mut self, _name: &str) -> RtResult<bool> {
        Err(RtError::unsupported_builtin("os.envHas"))
    }

    fn os_env_get(&mut self, _name: &str) -> RtResult<RtString> {
        Err(RtError::unsupported_builtin("os.envGet"))
    }

    fn os_env_set(&mut self, _name: &str, _value: &str) -> RtResult<()> {
        Err(RtError::unsupported_builtin("os.envSet"))
    }

    fn os_env_remove(&mut self, _name: &str) -> RtResult<()> {
        Err(RtError::unsupported_builtin("os.envRemove"))
    }

    fn os_sleep(&mut self, _millis: i64) -> RtResult<()> {
        Err(RtError::unsupported_builtin("os.sleep"))
    }

    fn os_exit(&mut self, _code: i64) -> RtResult<()> {
        Err(RtError::unsupported_builtin("os.exit"))
    }

    fn os_exec(&mut self, _program: &str, _args: &[String]) -> RtResult<i64> {
        Err(RtError::unsupported_builtin("os.exec"))
    }

    fn os_exec_out(&mut self, _program: &str, _args: &[String]) -> RtResult<RtString> {
        Err(RtError::unsupported_builtin("os.execOut"))
    }

    fn net_make_socket_handle(&mut self, _id: usize) -> RtResult<RtHandle> {
        Err(RtError::unsupported_builtin("net.Socket"))
    }

    fn net_make_listener_handle(&mut self, _id: usize) -> RtResult<RtHandle> {
        Err(RtError::unsupported_builtin("net.Listener"))
    }

    fn net_alloc_handle(&mut self, _kind: RtHandleKind) -> RtResult<RtHandle> {
        Err(RtError::unsupported_builtin("net.Handle"))
    }

    fn net_lookup_handle_kind(&mut self, _handle: RtHandle) -> RtResult<RtHandleKind> {
        Err(RtError::unsupported_builtin("net.Handle"))
    }

    fn net_close_handle(&mut self, _handle: RtHandle) -> RtResult<()> {
        Err(RtError::unsupported_builtin("net.Handle"))
    }

    fn net_store_tcp_stream(&mut self, _stream: TcpStream) -> RtResult<RtHandle> {
        Err(RtError::unsupported_builtin("net.Socket"))
    }

    fn net_store_tcp_listener(&mut self, _listener: TcpListener) -> RtResult<RtHandle> {
        Err(RtError::unsupported_builtin("net.Listener"))
    }

    fn net_tcp_stream(&mut self, _handle: RtHandle) -> RtResult<&mut TcpStream> {
        Err(RtError::unsupported_builtin("net.Socket"))
    }

    fn net_tcp_listener(&mut self, _handle: RtHandle) -> RtResult<&mut TcpListener> {
        Err(RtError::unsupported_builtin("net.Listener"))
    }
}

pub struct NoopHost {
    random_state: u64,
    net_resources: RtNetResourceTable,
}

impl Default for NoopHost {
    fn default() -> Self {
        Self {
            random_state: 0x1234_5678_9ABC_DEF0,
            net_resources: RtNetResourceTable::default(),
        }
    }
}

impl RtHost for NoopHost {
    fn io_print(&mut self, text: &str) -> RtResult<()> {
        print!("{text}");
        std::io::stdout()
            .flush()
            .map_err(|err| RtError::io(err.to_string()))?;
        Ok(())
    }

    fn datetime_now_unix(&mut self) -> RtResult<i64> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|err| RtError::new(RtErrorKind::InvalidArgument, err.to_string()))?;
        Ok(now.as_secs() as i64)
    }

    fn datetime_now_millis(&mut self) -> RtResult<i64> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|err| RtError::new(RtErrorKind::InvalidArgument, err.to_string()))?;
        Ok(now.as_millis() as i64)
    }

    fn datetime_from_unix(&mut self, value: i64) -> RtResult<RtString> {
        format_unix_timestamp(value, 0)
    }

    fn datetime_from_millis(&mut self, value: i64) -> RtResult<RtString> {
        format_unix_timestamp(value.div_euclid(1000), value.rem_euclid(1000) as u32)
    }

    fn datetime_parse_unix(&mut self, value: &str) -> RtResult<i64> {
        parse_iso8601_utc(value)
    }

    fn datetime_component(&mut self, name: &str, value: i64) -> RtResult<i64> {
        let (year, month, day, hour, minute, second) = unix_seconds_to_components(value);
        match name {
            "year" => Ok(year as i64),
            "month" => Ok(month as i64),
            "day" => Ok(day as i64),
            "hour" => Ok(hour as i64),
            "minute" => Ok(minute as i64),
            "second" => Ok(second as i64),
            _ => Err(RtError::new(
                RtErrorKind::InvalidArgument,
                format!("unknown datetime component `{name}`"),
            )),
        }
    }

    fn random_seed(&mut self, seed: i64) -> RtResult<()> {
        self.random_state = seed as u64;
        Ok(())
    }

    fn random_int(&mut self, min: i64, max: i64) -> RtResult<i64> {
        if min > max {
            return Err(RtError::new(
                RtErrorKind::InvalidArgument,
                "random.int min must be <= max",
            ));
        }
        self.random_state = self
            .random_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        let span = (max - min + 1) as u64;
        Ok(min + (self.random_state % span) as i64)
    }

    fn random_float(&mut self) -> RtResult<f64> {
        self.random_state = self
            .random_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        Ok((self.random_state as f64) / (u64::MAX as f64))
    }

    fn fs_exists(&mut self, path: &str) -> RtResult<bool> {
        Ok(PathBuf::from(path).exists())
    }

    fn fs_read_text(&mut self, path: &str) -> RtResult<RtString> {
        let text = fs::read_to_string(path).map_err(|err| RtError::io(err.to_string()))?;
        Ok(RtString::from(text))
    }

    fn fs_write_text(&mut self, path: &str, text: &str) -> RtResult<()> {
        fs::write(path, text).map_err(|err| RtError::io(err.to_string()))
    }

    fn fs_append_text(&mut self, path: &str, text: &str) -> RtResult<()> {
        use std::io::Write as _;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|err| RtError::io(err.to_string()))?;
        file.write_all(text.as_bytes())
            .map_err(|err| RtError::io(err.to_string()))
    }

    fn fs_mkdir_all(&mut self, path: &str) -> RtResult<()> {
        fs::create_dir_all(path).map_err(|err| RtError::io(err.to_string()))
    }

    fn fs_remove_file(&mut self, path: &str) -> RtResult<()> {
        fs::remove_file(path).map_err(|err| RtError::io(err.to_string()))
    }

    fn fs_remove_dir_all(&mut self, path: &str) -> RtResult<()> {
        fs::remove_dir_all(path).map_err(|err| RtError::io(err.to_string()))
    }

    fn fs_join(&mut self, left: &str, right: &str) -> RtResult<RtString> {
        Ok(RtString::from(
            PathBuf::from(left)
                .join(right)
                .to_string_lossy()
                .into_owned(),
        ))
    }

    fn os_platform(&mut self) -> RtResult<RtString> {
        Ok(RtString::from(std::env::consts::OS))
    }

    fn os_arch(&mut self) -> RtResult<RtString> {
        Ok(RtString::from(std::env::consts::ARCH))
    }

    fn os_arg(&mut self, index: i64) -> RtResult<RtString> {
        let index = usize::try_from(index).map_err(|_| {
            RtError::new(
                RtErrorKind::InvalidArgument,
                "os.arg index must be non-negative",
            )
        })?;
        let args = std::env::args().collect::<Vec<_>>();
        args.get(index)
            .cloned()
            .map(RtString::from)
            .ok_or_else(|| RtError::index_out_of_bounds(index, args.len()))
    }

    fn os_env_has(&mut self, name: &str) -> RtResult<bool> {
        Ok(std::env::var_os(name).is_some())
    }

    fn os_env_get(&mut self, name: &str) -> RtResult<RtString> {
        std::env::var(name).map(RtString::from).map_err(|_| {
            RtError::new(
                RtErrorKind::InvalidArgument,
                format!("environment variable `{name}` is not set or not valid UTF-8"),
            )
        })
    }

    fn os_env_set(&mut self, name: &str, value: &str) -> RtResult<()> {
        // Safety: tests and runtime mutate process environment synchronously through the host boundary.
        unsafe { std::env::set_var(name, value) };
        Ok(())
    }

    fn os_env_remove(&mut self, name: &str) -> RtResult<()> {
        // Safety: tests and runtime mutate process environment synchronously through the host boundary.
        unsafe { std::env::remove_var(name) };
        Ok(())
    }

    fn os_sleep(&mut self, millis: i64) -> RtResult<()> {
        if millis < 0 {
            return Err(RtError::new(
                RtErrorKind::InvalidArgument,
                "os.sleep millis must be non-negative",
            ));
        }
        std::thread::sleep(std::time::Duration::from_millis(millis as u64));
        Ok(())
    }

    fn os_exit(&mut self, code: i64) -> RtResult<()> {
        std::process::exit(code as i32)
    }

    fn os_exec(&mut self, program: &str, args: &[String]) -> RtResult<i64> {
        let output = Command::new(program)
            .args(args)
            .output()
            .map_err(|err| RtError::process(err.to_string()))?;
        Ok(output.status.code().unwrap_or(-1) as i64)
    }

    fn os_exec_out(&mut self, program: &str, args: &[String]) -> RtResult<RtString> {
        let output = Command::new(program)
            .args(args)
            .output()
            .map_err(|err| RtError::process(err.to_string()))?;
        Ok(RtString::from(
            String::from_utf8_lossy(&output.stdout)
                .trim_end()
                .to_string(),
        ))
    }

    fn net_make_socket_handle(&mut self, id: usize) -> RtResult<RtHandle> {
        let handle = RtHandle {
            id,
            kind: RtHandleKind::Socket,
        };
        self.net_resources
            .resources
            .insert(id, RtNetResource::Placeholder(RtHandleKind::Socket));
        Ok(handle)
    }

    fn net_make_listener_handle(&mut self, id: usize) -> RtResult<RtHandle> {
        let handle = RtHandle {
            id,
            kind: RtHandleKind::Listener,
        };
        self.net_resources
            .resources
            .insert(id, RtNetResource::Placeholder(RtHandleKind::Listener));
        Ok(handle)
    }

    fn net_alloc_handle(&mut self, kind: RtHandleKind) -> RtResult<RtHandle> {
        Ok(self.net_resources.alloc_placeholder(kind))
    }

    fn net_lookup_handle_kind(&mut self, handle: RtHandle) -> RtResult<RtHandleKind> {
        self.net_resources.kind_of(handle)
    }

    fn net_close_handle(&mut self, handle: RtHandle) -> RtResult<()> {
        self.net_resources.remove(handle)?;
        Ok(())
    }

    fn net_store_tcp_stream(&mut self, stream: TcpStream) -> RtResult<RtHandle> {
        Ok(self.net_resources.insert_socket(stream))
    }

    fn net_store_tcp_listener(&mut self, listener: TcpListener) -> RtResult<RtHandle> {
        Ok(self.net_resources.insert_listener(listener))
    }

    fn net_tcp_stream(&mut self, handle: RtHandle) -> RtResult<&mut TcpStream> {
        self.net_resources.socket_mut(handle)
    }

    fn net_tcp_listener(&mut self, handle: RtHandle) -> RtResult<&mut TcpListener> {
        self.net_resources.listener_mut(handle)
    }
}

fn format_unix_timestamp(seconds: i64, millis: u32) -> RtResult<RtString> {
    let (year, month, day, hour, minute, second) = unix_seconds_to_components(seconds);
    let text = if millis == 0 {
        format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
    } else {
        format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z")
    };
    Ok(RtString::from(text))
}

fn parse_iso8601_utc(value: &str) -> RtResult<i64> {
    let (date, time) = value.split_once('T').ok_or_else(|| {
        RtError::new(
            RtErrorKind::InvalidArgument,
            "datetime.parseUnix expects ISO-8601 UTC like `1970-01-01T00:00:00Z`",
        )
    })?;
    let time = time.strip_suffix('Z').ok_or_else(|| {
        RtError::new(
            RtErrorKind::InvalidArgument,
            "datetime.parseUnix expects trailing `Z`",
        )
    })?;

    let mut date_parts = date.split('-');
    let year: i32 = parse_part(date_parts.next(), "year")?;
    let month: u32 = parse_part(date_parts.next(), "month")?;
    let day: u32 = parse_part(date_parts.next(), "day")?;
    if date_parts.next().is_some() {
        return Err(RtError::new(
            RtErrorKind::InvalidArgument,
            "datetime.parseUnix date has too many parts",
        ));
    }

    let mut time_parts = time.split(':');
    let hour: u32 = parse_part(time_parts.next(), "hour")?;
    let minute: u32 = parse_part(time_parts.next(), "minute")?;
    let second: u32 = parse_part(time_parts.next(), "second")?;
    if time_parts.next().is_some() {
        return Err(RtError::new(
            RtErrorKind::InvalidArgument,
            "datetime.parseUnix time has too many parts",
        ));
    }

    if !(1..=12).contains(&month)
        || day == 0
        || day > days_in_month(year, month)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return Err(RtError::new(
            RtErrorKind::InvalidArgument,
            "datetime.parseUnix received out-of-range date/time component",
        ));
    }

    let days = days_from_civil(year, month, day);
    Ok(days * 86_400 + (hour as i64 * 3_600) + (minute as i64 * 60) + second as i64)
}

fn parse_part<T: std::str::FromStr>(value: Option<&str>, name: &str) -> RtResult<T> {
    value
        .ok_or_else(|| {
            RtError::new(
                RtErrorKind::InvalidArgument,
                format!("datetime.parseUnix missing {name}"),
            )
        })?
        .parse()
        .map_err(|_| {
            RtError::new(
                RtErrorKind::InvalidArgument,
                format!("datetime.parseUnix invalid {name}"),
            )
        })
}

fn unix_seconds_to_components(seconds: i64) -> (i32, u32, u32, u32, u32, u32) {
    let days = seconds.div_euclid(86_400);
    let secs_of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = (secs_of_day / 3_600) as u32;
    let minute = ((secs_of_day % 3_600) / 60) as u32;
    let second = (secs_of_day % 60) as u32;
    (year, month, day, hour, minute, second)
}

fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if m <= 2 { 1 } else { 0 };
    (year as i32, m as u32, d as u32)
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = year as i64 - if month <= 2 { 1 } else { 0 };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let month = month as i64;
    let day = day as i64;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}
