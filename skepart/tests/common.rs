#![allow(dead_code)]

use std::collections::HashMap;

use skepart::{RtBytes, RtError, RtHandle, RtHandleKind, RtHost, RtResult, RtString};

#[derive(Default)]
pub struct RecordingHost {
    pub output: String,
    pub unix_now: i64,
    pub millis_now: i64,
    pub random_int_value: i64,
    pub random_float_value: f64,
    pub platform: String,
    pub arch: String,
    pub args: Vec<String>,
    pub read_line: String,
    pub exec_status: i64,
    pub exec_out: String,
    pub exec_argv: Vec<String>,
    pub net_read_value: String,
    pub net_read_bytes_value: Vec<u8>,
    pub net_read_n_value: Vec<u8>,
    pub net_local_addr_value: String,
    pub net_peer_addr_value: String,
    pub net_flush_error: Option<String>,
    pub net_set_read_timeout_error: Option<String>,
    pub net_set_write_timeout_error: Option<String>,
    pub net_listen_error: Option<String>,
    pub net_connect_error: Option<String>,
    pub net_tls_connect_error: Option<String>,
    pub net_accept_error: Option<String>,
    pub net_read_error: Option<String>,
    pub net_write_error: Option<String>,
    pub next_handle_id: usize,
    pub net_handles: HashMap<usize, RtHandleKind>,
    pub env: HashMap<String, String>,
    pub files: HashMap<String, String>,
    pub existing_paths: HashMap<String, bool>,
}

impl RecordingHost {
    pub fn seeded() -> Self {
        Self {
            unix_now: 100,
            millis_now: 1234,
            random_int_value: 5,
            random_float_value: 0.25,
            platform: "test-os".into(),
            arch: "test-arch".into(),
            args: vec!["skepa".into(), "--flag".into()],
            read_line: "typed line".into(),
            exec_status: 9,
            exec_out: "exec-out".into(),
            exec_argv: Vec::new(),
            net_read_value: "net-read".into(),
            net_read_bytes_value: b"net-bytes".to_vec(),
            net_read_n_value: b"net-read-n".to_vec(),
            net_local_addr_value: "127.0.0.1:1111".into(),
            net_peer_addr_value: "127.0.0.1:2222".into(),
            next_handle_id: 0,
            net_handles: HashMap::new(),
            env: HashMap::from([(String::from("HOME"), String::from("/tmp/home"))]),
            files: HashMap::from([(String::from("exists.txt"), String::from("seeded"))]),
            existing_paths: HashMap::from([(String::from("exists.txt"), true)]),
            ..Self::default()
        }
    }
}

#[derive(Default)]
pub struct RecordingHostBuilder {
    host: RecordingHost,
}

impl RecordingHostBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn seeded() -> Self {
        Self {
            host: RecordingHost::seeded(),
        }
    }

    pub fn unix_now(mut self, value: i64) -> Self {
        self.host.unix_now = value;
        self
    }

    pub fn millis_now(mut self, value: i64) -> Self {
        self.host.millis_now = value;
        self
    }

    pub fn random_int(mut self, value: i64) -> Self {
        self.host.random_int_value = value;
        self
    }

    pub fn random_float(mut self, value: f64) -> Self {
        self.host.random_float_value = value;
        self
    }

    pub fn platform(mut self, value: impl Into<String>) -> Self {
        self.host.platform = value.into();
        self
    }

    pub fn arch(mut self, value: impl Into<String>) -> Self {
        self.host.arch = value.into();
        self
    }

    pub fn args(mut self, values: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.host.args = values.into_iter().map(Into::into).collect();
        self
    }

    pub fn read_line(mut self, value: impl Into<String>) -> Self {
        self.host.read_line = value.into();
        self
    }

    pub fn exec_status(mut self, value: i64) -> Self {
        self.host.exec_status = value;
        self
    }

    pub fn exec_out(mut self, value: impl Into<String>) -> Self {
        self.host.exec_out = value.into();
        self
    }

    pub fn exec_argv(mut self, values: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.host.exec_argv = values.into_iter().map(Into::into).collect();
        self
    }

    pub fn net_read_value(mut self, value: impl Into<String>) -> Self {
        self.host.net_read_value = value.into();
        self
    }

    pub fn net_read_bytes_value(mut self, value: impl Into<Vec<u8>>) -> Self {
        self.host.net_read_bytes_value = value.into();
        self
    }

    pub fn net_read_n_value(mut self, value: impl Into<Vec<u8>>) -> Self {
        self.host.net_read_n_value = value.into();
        self
    }

    pub fn net_local_addr_value(mut self, value: impl Into<String>) -> Self {
        self.host.net_local_addr_value = value.into();
        self
    }

    pub fn net_peer_addr_value(mut self, value: impl Into<String>) -> Self {
        self.host.net_peer_addr_value = value.into();
        self
    }

    pub fn net_flush_error(mut self, value: impl Into<String>) -> Self {
        self.host.net_flush_error = Some(value.into());
        self
    }

    pub fn net_set_read_timeout_error(mut self, value: impl Into<String>) -> Self {
        self.host.net_set_read_timeout_error = Some(value.into());
        self
    }

    pub fn net_set_write_timeout_error(mut self, value: impl Into<String>) -> Self {
        self.host.net_set_write_timeout_error = Some(value.into());
        self
    }

    pub fn net_listen_error(mut self, value: impl Into<String>) -> Self {
        self.host.net_listen_error = Some(value.into());
        self
    }

    pub fn net_connect_error(mut self, value: impl Into<String>) -> Self {
        self.host.net_connect_error = Some(value.into());
        self
    }

    pub fn net_tls_connect_error(mut self, value: impl Into<String>) -> Self {
        self.host.net_tls_connect_error = Some(value.into());
        self
    }

    pub fn net_accept_error(mut self, value: impl Into<String>) -> Self {
        self.host.net_accept_error = Some(value.into());
        self
    }

    pub fn net_read_error(mut self, value: impl Into<String>) -> Self {
        self.host.net_read_error = Some(value.into());
        self
    }

    pub fn net_write_error(mut self, value: impl Into<String>) -> Self {
        self.host.net_write_error = Some(value.into());
        self
    }

    pub fn file(mut self, path: impl Into<String>, contents: impl Into<String>) -> Self {
        let path = path.into();
        self.host.files.insert(path.clone(), contents.into());
        self.host.existing_paths.insert(path, true);
        self
    }

    pub fn existing_path(mut self, path: impl Into<String>, exists: bool) -> Self {
        self.host.existing_paths.insert(path.into(), exists);
        self
    }

    pub fn env(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.host.env.insert(name.into(), value.into());
        self
    }

    pub fn build(self) -> RecordingHost {
        self.host
    }
}

impl RtHost for RecordingHost {
    fn io_print(&mut self, text: &str) -> RtResult<()> {
        self.output.push_str(text);
        Ok(())
    }

    fn io_read_line(&mut self) -> RtResult<RtString> {
        Ok(RtString::from(self.read_line.clone()))
    }

    fn datetime_now_unix(&mut self) -> RtResult<i64> {
        Ok(self.unix_now)
    }

    fn datetime_now_millis(&mut self) -> RtResult<i64> {
        Ok(self.millis_now)
    }

    fn datetime_from_unix(&mut self, value: i64) -> RtResult<RtString> {
        Ok(RtString::from(format!("unix:{value}")))
    }

    fn datetime_from_millis(&mut self, value: i64) -> RtResult<RtString> {
        Ok(RtString::from(format!("millis:{value}")))
    }

    fn datetime_parse_unix(&mut self, value: &str) -> RtResult<i64> {
        Ok(value.len() as i64)
    }

    fn datetime_component(&mut self, name: &str, value: i64) -> RtResult<i64> {
        Ok(value + name.len() as i64)
    }

    fn random_seed(&mut self, _seed: i64) -> RtResult<()> {
        Ok(())
    }

    fn random_int(&mut self, _min: i64, _max: i64) -> RtResult<i64> {
        Ok(self.random_int_value)
    }

    fn random_float(&mut self) -> RtResult<f64> {
        Ok(self.random_float_value)
    }

    fn fs_exists(&mut self, path: &str) -> RtResult<bool> {
        Ok(self.existing_paths.get(path).copied().unwrap_or(false))
    }

    fn fs_read_text(&mut self, path: &str) -> RtResult<RtString> {
        Ok(RtString::from(
            self.files
                .get(path)
                .cloned()
                .unwrap_or_else(|| format!("read:{path}")),
        ))
    }

    fn fs_write_text(&mut self, path: &str, text: &str) -> RtResult<()> {
        self.files.insert(path.to_string(), text.to_string());
        self.existing_paths.insert(path.to_string(), true);
        self.output.push_str(&format!("[write {path}={text}]"));
        Ok(())
    }

    fn fs_append_text(&mut self, path: &str, text: &str) -> RtResult<()> {
        self.files
            .entry(path.to_string())
            .and_modify(|existing| existing.push_str(text))
            .or_insert_with(|| text.to_string());
        self.existing_paths.insert(path.to_string(), true);
        self.output.push_str(&format!("[append {path}+={text}]"));
        Ok(())
    }

    fn fs_mkdir_all(&mut self, path: &str) -> RtResult<()> {
        self.existing_paths.insert(path.to_string(), true);
        self.output.push_str(&format!("[mkdir {path}]"));
        Ok(())
    }

    fn fs_remove_file(&mut self, path: &str) -> RtResult<()> {
        self.files.remove(path);
        self.existing_paths.insert(path.to_string(), false);
        self.output.push_str(&format!("[rmfile {path}]"));
        Ok(())
    }

    fn fs_remove_dir_all(&mut self, path: &str) -> RtResult<()> {
        self.existing_paths.insert(path.to_string(), false);
        self.output.push_str(&format!("[rmdir {path}]"));
        Ok(())
    }

    fn fs_join(&mut self, left: &str, right: &str) -> RtResult<RtString> {
        Ok(RtString::from(format!("{left}/{right}")))
    }

    fn os_platform(&mut self) -> RtResult<RtString> {
        Ok(RtString::from(self.platform.clone()))
    }

    fn os_arch(&mut self) -> RtResult<RtString> {
        Ok(RtString::from(self.arch.clone()))
    }

    fn os_arg(&mut self, index: i64) -> RtResult<RtString> {
        let index = usize::try_from(index).map_err(|_| {
            skepart::RtError::new(
                skepart::RtErrorKind::InvalidArgument,
                "os.arg index must be non-negative",
            )
        })?;
        self.args
            .get(index)
            .cloned()
            .map(RtString::from)
            .ok_or_else(|| skepart::RtError::index_out_of_bounds(index, self.args.len()))
    }

    fn os_env_has(&mut self, name: &str) -> RtResult<bool> {
        Ok(self.env.contains_key(name))
    }

    fn os_env_get(&mut self, name: &str) -> RtResult<RtString> {
        self.env
            .get(name)
            .cloned()
            .map(RtString::from)
            .ok_or_else(|| {
                skepart::RtError::new(
                    skepart::RtErrorKind::InvalidArgument,
                    format!("environment variable `{name}` is not set or not valid UTF-8"),
                )
            })
    }

    fn os_env_set(&mut self, name: &str, value: &str) -> RtResult<()> {
        self.env.insert(name.to_string(), value.to_string());
        self.output.push_str(&format!("[envset {name}={value}]"));
        Ok(())
    }

    fn os_env_remove(&mut self, name: &str) -> RtResult<()> {
        self.env.remove(name);
        self.output.push_str(&format!("[envrm {name}]"));
        Ok(())
    }

    fn os_sleep(&mut self, millis: i64) -> RtResult<()> {
        self.output.push_str(&format!("[sleep {millis}]"));
        Ok(())
    }

    fn os_exit(&mut self, code: i64) -> RtResult<()> {
        self.output.push_str(&format!("[exit {code}]"));
        Ok(())
    }

    fn os_exec(&mut self, program: &str, args: &[String]) -> RtResult<i64> {
        self.exec_argv = args.to_vec();
        if args.is_empty() {
            self.output.push_str(&format!("[exec {program}]"));
        } else {
            self.output
                .push_str(&format!("[exec {program} {}]", args.join(" ")));
        }
        Ok(self.exec_status)
    }

    fn os_exec_out(&mut self, program: &str, args: &[String]) -> RtResult<RtString> {
        self.exec_argv = args.to_vec();
        if args.is_empty() {
            self.output.push_str(&format!("[execout {program}]"));
        } else {
            self.output
                .push_str(&format!("[execout {program} {}]", args.join(" ")));
        }
        Ok(RtString::from(self.exec_out.clone()))
    }

    fn net_make_socket_handle(&mut self, id: usize) -> RtResult<RtHandle> {
        self.net_handles.insert(id, RtHandleKind::Socket);
        Ok(RtHandle {
            id,
            kind: RtHandleKind::Socket,
        })
    }

    fn net_make_listener_handle(&mut self, id: usize) -> RtResult<RtHandle> {
        self.net_handles.insert(id, RtHandleKind::Listener);
        Ok(RtHandle {
            id,
            kind: RtHandleKind::Listener,
        })
    }

    fn task_make_task_handle(&mut self, id: usize) -> RtResult<RtHandle> {
        self.net_handles.insert(id, RtHandleKind::Task);
        Ok(RtHandle {
            id,
            kind: RtHandleKind::Task,
        })
    }

    fn task_make_channel_handle(&mut self, id: usize) -> RtResult<RtHandle> {
        self.net_handles.insert(id, RtHandleKind::Channel);
        Ok(RtHandle {
            id,
            kind: RtHandleKind::Channel,
        })
    }

    fn net_alloc_handle(&mut self, kind: RtHandleKind) -> RtResult<RtHandle> {
        let handle = RtHandle {
            id: self.next_handle_id,
            kind,
        };
        self.next_handle_id += 1;
        self.net_handles.insert(handle.id, kind);
        Ok(handle)
    }

    fn net_lookup_handle_kind(&mut self, handle: RtHandle) -> RtResult<RtHandleKind> {
        let actual = self.net_handles.get(&handle.id).copied().ok_or_else(|| {
            skepart::RtError::invalid_handle(format!("unknown handle id {}", handle.id))
        })?;
        if actual != handle.kind {
            return Err(skepart::RtError::invalid_handle_kind(
                handle.kind.type_name(),
                actual.type_name(),
            ));
        }
        Ok(actual)
    }

    fn net_close_handle(&mut self, handle: RtHandle) -> RtResult<()> {
        self.net_lookup_handle_kind(handle)?;
        self.net_handles.remove(&handle.id);
        Ok(())
    }

    fn net_listen(&mut self, _address: &str) -> RtResult<RtHandle> {
        if let Some(message) = &self.net_listen_error {
            return Err(RtError::io(message.clone()));
        }
        self.net_alloc_handle(RtHandleKind::Listener)
    }

    fn net_connect(&mut self, _address: &str) -> RtResult<RtHandle> {
        if let Some(message) = &self.net_connect_error {
            return Err(RtError::io(message.clone()));
        }
        self.net_alloc_handle(RtHandleKind::Socket)
    }

    fn net_tls_connect(&mut self, host: &str, port: i64) -> RtResult<RtHandle> {
        if let Some(message) = &self.net_tls_connect_error {
            return Err(RtError::io(message.clone()));
        }
        if port < 0 {
            return Err(RtError::new(
                skepart::RtErrorKind::InvalidArgument,
                "net.tlsConnect port must be non-negative",
            ));
        }
        let handle = self.net_alloc_handle(RtHandleKind::Socket)?;
        self.output
            .push_str(&format!("[nettlsconnect {}={host}:{port}]", handle.id));
        Ok(handle)
    }

    fn net_accept(&mut self, listener: RtHandle) -> RtResult<RtHandle> {
        if let Some(message) = &self.net_accept_error {
            return Err(RtError::io(message.clone()));
        }
        self.net_lookup_handle_kind(listener)?;
        self.net_alloc_handle(RtHandleKind::Socket)
    }

    fn net_read(&mut self, socket: RtHandle) -> RtResult<RtString> {
        if let Some(message) = &self.net_read_error {
            return Err(RtError::io(message.clone()));
        }
        self.net_lookup_handle_kind(socket)?;
        self.output.push_str(&format!("[netread {}]", socket.id));
        Ok(RtString::from(self.net_read_value.clone()))
    }

    fn net_write(&mut self, socket: RtHandle, data: &str) -> RtResult<()> {
        if let Some(message) = &self.net_write_error {
            return Err(RtError::io(message.clone()));
        }
        self.net_lookup_handle_kind(socket)?;
        self.output
            .push_str(&format!("[netwrite {}={data}]", socket.id));
        Ok(())
    }

    fn net_read_bytes(&mut self, socket: RtHandle) -> RtResult<RtBytes> {
        if let Some(message) = &self.net_read_error {
            return Err(RtError::io(message.clone()));
        }
        self.net_lookup_handle_kind(socket)?;
        self.output
            .push_str(&format!("[netreadbytes {}]", socket.id));
        Ok(RtBytes::from(self.net_read_bytes_value.clone()))
    }

    fn net_write_bytes(&mut self, socket: RtHandle, data: &RtBytes) -> RtResult<()> {
        if let Some(message) = &self.net_write_error {
            return Err(RtError::io(message.clone()));
        }
        self.net_lookup_handle_kind(socket)?;
        self.output
            .push_str(&format!("[netwritebytes {} len={}]", socket.id, data.len()));
        Ok(())
    }

    fn net_read_n(&mut self, socket: RtHandle, count: i64) -> RtResult<RtBytes> {
        if let Some(message) = &self.net_read_error {
            return Err(RtError::io(message.clone()));
        }
        let count = usize::try_from(count).map_err(|_| {
            RtError::new(
                skepart::RtErrorKind::InvalidArgument,
                "net.readN count must be non-negative",
            )
        })?;
        self.net_lookup_handle_kind(socket)?;
        self.output
            .push_str(&format!("[netreadn {} count={}]", socket.id, count));
        if self.net_read_n_value.len() < count {
            return Err(RtError::io("not enough bytes available".to_string()));
        }
        Ok(RtBytes::from(self.net_read_n_value[..count].to_vec()))
    }

    fn net_local_addr(&mut self, socket: RtHandle) -> RtResult<RtString> {
        self.net_lookup_handle_kind(socket)?;
        self.output
            .push_str(&format!("[netlocaladdr {}]", socket.id));
        Ok(RtString::from(self.net_local_addr_value.clone()))
    }

    fn net_peer_addr(&mut self, socket: RtHandle) -> RtResult<RtString> {
        self.net_lookup_handle_kind(socket)?;
        self.output
            .push_str(&format!("[netpeeraddr {}]", socket.id));
        Ok(RtString::from(self.net_peer_addr_value.clone()))
    }

    fn net_flush(&mut self, socket: RtHandle) -> RtResult<()> {
        if let Some(message) = &self.net_flush_error {
            return Err(RtError::io(message.clone()));
        }
        self.net_lookup_handle_kind(socket)?;
        self.output.push_str(&format!("[netflush {}]", socket.id));
        Ok(())
    }

    fn net_set_read_timeout(&mut self, socket: RtHandle, millis: i64) -> RtResult<()> {
        if let Some(message) = &self.net_set_read_timeout_error {
            return Err(RtError::io(message.clone()));
        }
        if millis < 0 {
            return Err(RtError::new(
                skepart::RtErrorKind::InvalidArgument,
                "net.setReadTimeout millis must be non-negative",
            ));
        }
        self.net_lookup_handle_kind(socket)?;
        self.output
            .push_str(&format!("[netsetreadtimeout {}={}]", socket.id, millis));
        Ok(())
    }

    fn net_set_write_timeout(&mut self, socket: RtHandle, millis: i64) -> RtResult<()> {
        if let Some(message) = &self.net_set_write_timeout_error {
            return Err(RtError::io(message.clone()));
        }
        if millis < 0 {
            return Err(RtError::new(
                skepart::RtErrorKind::InvalidArgument,
                "net.setWriteTimeout millis must be non-negative",
            ));
        }
        self.net_lookup_handle_kind(socket)?;
        self.output
            .push_str(&format!("[netsetwritetimeout {}={}]", socket.id, millis));
        Ok(())
    }
}
