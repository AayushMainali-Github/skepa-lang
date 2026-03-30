use std::collections::HashMap;
use std::collections::VecDeque;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use rustls::pki_types::{CertificateDer, ServerName};
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};

use crate::{
    RtBytes, RtError, RtErrorKind, RtHandle, RtHandleKind, RtMap, RtResult, RtString, RtValue,
};

pub enum RtNetResource {
    Placeholder(RtHandleKind),
    Task(Arc<Mutex<RtTaskState>>),
    Channel(Arc<Mutex<VecDeque<crate::RtValue>>>),
    TcpStream(TcpStream),
    TlsStream(Box<StreamOwned<ClientConnection, TcpStream>>),
    TcpListener(TcpListener),
}

pub enum RtTaskState {
    Completed(Option<crate::RtValue>),
    Running(Option<JoinHandle<RtResult<crate::RtValue>>>),
}

impl RtNetResource {
    pub fn kind(&self) -> RtHandleKind {
        match self {
            Self::Placeholder(kind) => *kind,
            Self::Task(_) => RtHandleKind::Task,
            Self::Channel(_) => RtHandleKind::Channel,
            Self::TcpStream(_) | Self::TlsStream(_) => RtHandleKind::Socket,
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

    pub fn insert_tls_socket(
        &mut self,
        stream: StreamOwned<ClientConnection, TcpStream>,
    ) -> RtHandle {
        self.insert(RtNetResource::TlsStream(Box::new(stream)))
    }

    pub fn insert_listener(&mut self, listener: TcpListener) -> RtHandle {
        self.insert(RtNetResource::TcpListener(listener))
    }

    pub fn insert_channel(&mut self) -> RtHandle {
        self.insert(RtNetResource::Channel(Arc::new(
            Mutex::new(VecDeque::new()),
        )))
    }

    pub fn insert_task(&mut self, value: crate::RtValue) -> RtHandle {
        self.insert(RtNetResource::Task(Arc::new(Mutex::new(
            RtTaskState::Completed(Some(value)),
        ))))
    }

    pub fn insert_running_task(
        &mut self,
        handle: JoinHandle<RtResult<crate::RtValue>>,
    ) -> RtHandle {
        self.insert(RtNetResource::Task(Arc::new(Mutex::new(
            RtTaskState::Running(Some(handle)),
        ))))
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

    pub fn channel(&self, handle: RtHandle) -> RtResult<Arc<Mutex<VecDeque<crate::RtValue>>>> {
        self.kind_of(handle)?;
        match self.resources.get(&handle.id) {
            Some(RtNetResource::Channel(queue)) => Ok(Arc::clone(queue)),
            Some(other) => Err(RtError::invalid_handle_kind(
                RtHandleKind::Channel.type_name(),
                other.kind().type_name(),
            )),
            None => Err(RtError::invalid_handle(format!(
                "unknown handle id {}",
                handle.id
            ))),
        }
    }

    pub fn task(&self, handle: RtHandle) -> RtResult<Arc<Mutex<RtTaskState>>> {
        self.kind_of(handle)?;
        match self.resources.get(&handle.id) {
            Some(RtNetResource::Task(value)) => Ok(Arc::clone(value)),
            Some(other) => Err(RtError::invalid_handle_kind(
                RtHandleKind::Task.type_name(),
                other.kind().type_name(),
            )),
            None => Err(RtError::invalid_handle(format!(
                "unknown handle id {}",
                handle.id
            ))),
        }
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

    fn task_make_task_handle(&mut self, _id: usize) -> RtResult<RtHandle> {
        Err(RtError::unsupported_builtin("task.Task"))
    }

    fn task_make_channel_handle(&mut self, _id: usize) -> RtResult<RtHandle> {
        Err(RtError::unsupported_builtin("task.Channel"))
    }

    fn task_store_completed(&mut self, _value: crate::RtValue) -> RtResult<RtHandle> {
        Err(RtError::unsupported_builtin("task.Task"))
    }

    fn task_store_running(
        &mut self,
        _task: JoinHandle<RtResult<crate::RtValue>>,
    ) -> RtResult<RtHandle> {
        Err(RtError::unsupported_builtin("task.Task"))
    }

    fn task_channel(&mut self) -> RtResult<RtHandle> {
        Err(RtError::unsupported_builtin("task.channel"))
    }

    fn task_send(&mut self, _channel: RtHandle, _value: crate::RtValue) -> RtResult<()> {
        Err(RtError::unsupported_builtin("task.send"))
    }

    fn task_recv(&mut self, _channel: RtHandle) -> RtResult<crate::RtValue> {
        Err(RtError::unsupported_builtin("task.recv"))
    }

    fn task_join(&mut self, _task: RtHandle) -> RtResult<crate::RtValue> {
        Err(RtError::unsupported_builtin("task.join"))
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

    fn net_listen(&mut self, _address: &str) -> RtResult<RtHandle> {
        Err(RtError::unsupported_builtin("net.listen"))
    }

    fn net_connect(&mut self, _address: &str) -> RtResult<RtHandle> {
        Err(RtError::unsupported_builtin("net.connect"))
    }

    fn net_tls_connect(&mut self, _host: &str, _port: i64) -> RtResult<RtHandle> {
        Err(RtError::unsupported_builtin("net.tlsConnect"))
    }

    fn net_resolve(&mut self, _host: &str) -> RtResult<RtString> {
        Err(RtError::unsupported_builtin("net.resolve"))
    }

    fn net_parse_url(&mut self, _url: &str) -> RtResult<RtMap> {
        Err(RtError::unsupported_builtin("net.parseUrl"))
    }

    fn net_http_get(&mut self, _url: &str) -> RtResult<RtString> {
        Err(RtError::unsupported_builtin("net.httpGet"))
    }

    fn net_http_post(&mut self, _url: &str, _body: &str) -> RtResult<RtString> {
        Err(RtError::unsupported_builtin("net.httpPost"))
    }

    fn net_fetch(&mut self, _url: &str, _options: &RtMap) -> RtResult<RtMap> {
        Err(RtError::unsupported_builtin("net.fetch"))
    }

    fn net_accept(&mut self, _listener: RtHandle) -> RtResult<RtHandle> {
        Err(RtError::unsupported_builtin("net.accept"))
    }

    fn net_read(&mut self, _socket: RtHandle) -> RtResult<RtString> {
        Err(RtError::unsupported_builtin("net.read"))
    }

    fn net_write(&mut self, _socket: RtHandle, _data: &str) -> RtResult<()> {
        Err(RtError::unsupported_builtin("net.write"))
    }

    fn net_read_bytes(&mut self, _socket: RtHandle) -> RtResult<RtBytes> {
        Err(RtError::unsupported_builtin("net.readBytes"))
    }

    fn net_write_bytes(&mut self, _socket: RtHandle, _data: &RtBytes) -> RtResult<()> {
        Err(RtError::unsupported_builtin("net.writeBytes"))
    }

    fn net_read_n(&mut self, _socket: RtHandle, _count: i64) -> RtResult<RtBytes> {
        Err(RtError::unsupported_builtin("net.readN"))
    }

    fn net_local_addr(&mut self, _socket: RtHandle) -> RtResult<RtString> {
        Err(RtError::unsupported_builtin("net.localAddr"))
    }

    fn net_peer_addr(&mut self, _socket: RtHandle) -> RtResult<RtString> {
        Err(RtError::unsupported_builtin("net.peerAddr"))
    }

    fn net_flush(&mut self, _socket: RtHandle) -> RtResult<()> {
        Err(RtError::unsupported_builtin("net.flush"))
    }

    fn net_set_read_timeout(&mut self, _socket: RtHandle, _millis: i64) -> RtResult<()> {
        Err(RtError::unsupported_builtin("net.setReadTimeout"))
    }

    fn net_set_write_timeout(&mut self, _socket: RtHandle, _millis: i64) -> RtResult<()> {
        Err(RtError::unsupported_builtin("net.setWriteTimeout"))
    }
}

pub struct NoopHost {
    random_state: u64,
    net_resources: RtNetResourceTable,
    tls_root_certs: Vec<CertificateDer<'static>>,
}

impl Default for NoopHost {
    fn default() -> Self {
        Self {
            random_state: 0x1234_5678_9ABC_DEF0,
            net_resources: RtNetResourceTable::default(),
            tls_root_certs: Vec::new(),
        }
    }
}

impl NoopHost {
    pub fn add_tls_root_certificate(&mut self, cert: CertificateDer<'static>) {
        self.tls_root_certs.push(cert);
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

    fn task_make_task_handle(&mut self, id: usize) -> RtResult<RtHandle> {
        let handle = RtHandle {
            id,
            kind: RtHandleKind::Task,
        };
        self.net_resources
            .resources
            .insert(id, RtNetResource::Placeholder(RtHandleKind::Task));
        Ok(handle)
    }

    fn task_make_channel_handle(&mut self, id: usize) -> RtResult<RtHandle> {
        let handle = RtHandle {
            id,
            kind: RtHandleKind::Channel,
        };
        self.net_resources
            .resources
            .insert(id, RtNetResource::Placeholder(RtHandleKind::Channel));
        Ok(handle)
    }

    fn task_store_completed(&mut self, value: crate::RtValue) -> RtResult<RtHandle> {
        Ok(self.net_resources.insert_task(value))
    }

    fn task_store_running(
        &mut self,
        task: JoinHandle<RtResult<crate::RtValue>>,
    ) -> RtResult<RtHandle> {
        Ok(self.net_resources.insert_running_task(task))
    }

    fn task_channel(&mut self) -> RtResult<RtHandle> {
        Ok(self.net_resources.insert_channel())
    }

    fn task_send(&mut self, channel: RtHandle, value: crate::RtValue) -> RtResult<()> {
        let queue = self.net_resources.channel(channel)?;
        let mut queue = queue
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        queue.push_back(value);
        Ok(())
    }

    fn task_recv(&mut self, channel: RtHandle) -> RtResult<crate::RtValue> {
        let queue = self.net_resources.channel(channel)?;
        let mut queue = queue
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        queue.pop_front().ok_or_else(|| {
            RtError::new(
                RtErrorKind::InvalidArgument,
                "cannot receive from empty channel",
            )
        })
    }

    fn task_join(&mut self, task: RtHandle) -> RtResult<crate::RtValue> {
        let state = self.net_resources.task(task)?;
        let running = {
            let mut state = state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            match &mut *state {
                RtTaskState::Completed(value) => {
                    return value.take().ok_or_else(|| {
                        RtError::new(
                            RtErrorKind::InvalidArgument,
                            "cannot join completed task more than once",
                        )
                    });
                }
                RtTaskState::Running(handle) => handle.take().ok_or_else(|| {
                    RtError::new(
                        RtErrorKind::InvalidArgument,
                        "cannot join completed task more than once",
                    )
                })?,
            }
        };
        match running.join() {
            Ok(result) => result,
            Err(_) => Err(RtError::new(
                RtErrorKind::InvalidArgument,
                "spawned task panicked",
            )),
        }
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

    fn net_listen(&mut self, address: &str) -> RtResult<RtHandle> {
        let listener = TcpListener::bind(address).map_err(|err| RtError::io(err.to_string()))?;
        self.net_store_tcp_listener(listener)
    }

    fn net_connect(&mut self, address: &str) -> RtResult<RtHandle> {
        let stream = TcpStream::connect(address).map_err(|err| RtError::io(err.to_string()))?;
        self.net_store_tcp_stream(stream)
    }

    fn net_tls_connect(&mut self, host: &str, port: i64) -> RtResult<RtHandle> {
        let port = u16::try_from(port).map_err(|_| {
            RtError::new(
                RtErrorKind::InvalidArgument,
                "net.tlsConnect port must fit in 0..65535",
            )
        })?;
        let addr = format!("{host}:{port}");
        let tcp = TcpStream::connect(&addr).map_err(|err| RtError::io(err.to_string()))?;
        let server_name = ServerName::try_from(host.to_string()).map_err(|_| {
            RtError::new(
                RtErrorKind::InvalidArgument,
                format!("net.tlsConnect invalid hostname `{host}`"),
            )
        })?;
        let config = tls_client_config(&self.tls_root_certs)?;
        let mut tls = StreamOwned::new(
            ClientConnection::new(config, server_name)
                .map_err(|err| RtError::process(err.to_string()))?,
            tcp,
        );
        tls.conn
            .complete_io(&mut tls.sock)
            .map_err(|err| RtError::io(err.to_string()))?;
        Ok(self.net_resources.insert_tls_socket(tls))
    }

    fn net_resolve(&mut self, host: &str) -> RtResult<RtString> {
        let mut addrs = (host, 0)
            .to_socket_addrs()
            .map_err(|err| RtError::io(err.to_string()))?;
        let addr = addrs
            .next()
            .ok_or_else(|| RtError::io(format!("no addresses resolved for `{host}`")))?;
        Ok(RtString::from(addr.ip().to_string()))
    }

    fn net_parse_url(&mut self, url: &str) -> RtResult<RtMap> {
        let parts = parse_url_parts(url)?;
        let map = RtMap::new();
        map.insert("scheme", RtValue::String(RtString::from(parts.scheme)));
        map.insert("host", RtValue::String(RtString::from(parts.host)));
        map.insert("port", RtValue::String(RtString::from(parts.port)));
        map.insert("path", RtValue::String(RtString::from(parts.path)));
        map.insert("query", RtValue::String(RtString::from(parts.query)));
        map.insert("fragment", RtValue::String(RtString::from(parts.fragment)));
        Ok(map)
    }

    fn net_http_get(&mut self, url: &str) -> RtResult<RtString> {
        let response = self.http_request(url, "GET", "")?;
        Ok(RtString::from(response))
    }

    fn net_http_post(&mut self, url: &str, body: &str) -> RtResult<RtString> {
        let response = self.http_request(url, "POST", body)?;
        Ok(RtString::from(response))
    }

    fn net_fetch(&mut self, url: &str, options: &RtMap) -> RtResult<RtMap> {
        let method = match options.get("method") {
            Ok(value) => value.expect_string()?.as_str().to_owned(),
            Err(err) if err.kind == RtErrorKind::MissingField => "GET".to_string(),
            Err(err) => return Err(err),
        };
        let body = match options.get("body") {
            Ok(value) => value.expect_string()?.as_str().to_owned(),
            Err(err) if err.kind == RtErrorKind::MissingField => String::new(),
            Err(err) => return Err(err),
        };
        let content_type = match options.get("contentType") {
            Ok(value) => value.expect_string()?.as_str().to_owned(),
            Err(err) if err.kind == RtErrorKind::MissingField => String::new(),
            Err(err) => return Err(err),
        };

        let method = method.to_uppercase();
        let response = self.http_request_with_content_type(url, &method, &body, &content_type)?;
        let map = RtMap::new();
        map.insert("status", RtValue::String(RtString::from(response.status)));
        map.insert("body", RtValue::String(RtString::from(response.body)));
        map.insert(
            "contentType",
            RtValue::String(RtString::from(response.content_type)),
        );
        Ok(map)
    }

    fn net_accept(&mut self, listener: RtHandle) -> RtResult<RtHandle> {
        let stream = {
            let listener_ref = self.net_tcp_listener(listener)?;
            let (stream, _) = listener_ref
                .accept()
                .map_err(|err| RtError::io(err.to_string()))?;
            stream
        };
        self.net_store_tcp_stream(stream)
    }

    fn net_read(&mut self, socket: RtHandle) -> RtResult<RtString> {
        let mut buf = [0_u8; 4096];
        let bytes = self.socket_read(socket, &mut buf)?;
        // `net` is intentionally text-first for now; binary payloads will need a future bytes API.
        String::from_utf8(buf[..bytes].to_vec())
            .map(RtString::from)
            .map_err(|_| {
                RtError::new(
                    RtErrorKind::InvalidArgument,
                    "net.read expected valid UTF-8 data",
                )
            })
    }

    fn net_write(&mut self, socket: RtHandle, data: &str) -> RtResult<()> {
        self.socket_write_all(socket, data.as_bytes())
    }

    fn net_read_bytes(&mut self, socket: RtHandle) -> RtResult<RtBytes> {
        let mut buf = [0_u8; 4096];
        let bytes = self.socket_read(socket, &mut buf)?;
        Ok(RtBytes::from(buf[..bytes].to_vec()))
    }

    fn net_write_bytes(&mut self, socket: RtHandle, data: &RtBytes) -> RtResult<()> {
        self.socket_write_all(socket, data.as_slice())
    }

    fn net_read_n(&mut self, socket: RtHandle, count: i64) -> RtResult<RtBytes> {
        let count = usize::try_from(count).map_err(|_| {
            RtError::new(
                RtErrorKind::InvalidArgument,
                "net.readN count must be non-negative",
            )
        })?;
        let mut buf = vec![0_u8; count];
        self.socket_read_exact(socket, &mut buf)?;
        Ok(RtBytes::from(buf))
    }

    fn net_local_addr(&mut self, socket: RtHandle) -> RtResult<RtString> {
        let addr = self.socket_local_addr(socket)?;
        Ok(RtString::from(addr.to_string()))
    }

    fn net_peer_addr(&mut self, socket: RtHandle) -> RtResult<RtString> {
        let addr = self.socket_peer_addr(socket)?;
        Ok(RtString::from(addr.to_string()))
    }

    fn net_flush(&mut self, socket: RtHandle) -> RtResult<()> {
        self.socket_flush(socket)
    }

    fn net_set_read_timeout(&mut self, socket: RtHandle, millis: i64) -> RtResult<()> {
        let timeout = duration_from_timeout_millis("net.setReadTimeout", millis)?;
        self.socket_set_read_timeout(socket, timeout)
    }

    fn net_set_write_timeout(&mut self, socket: RtHandle, millis: i64) -> RtResult<()> {
        let timeout = duration_from_timeout_millis("net.setWriteTimeout", millis)?;
        self.socket_set_write_timeout(socket, timeout)
    }
}

impl NoopHost {
    fn http_request(&mut self, url: &str, method: &str, body: &str) -> RtResult<String> {
        self.http_request_with_content_type(url, method, body, "")
            .map(|response| response.body)
    }

    fn http_request_with_content_type(
        &mut self,
        url: &str,
        method: &str,
        body: &str,
        content_type: &str,
    ) -> RtResult<HttpResponseParts> {
        let parts = parse_url_parts(url)?;
        let port = if parts.port.is_empty() {
            match parts.scheme.as_str() {
                "http" => "80",
                "https" => "443",
                other => {
                    return Err(RtError::new(
                        RtErrorKind::InvalidArgument,
                        format!(
                            "net.http{} unsupported URL scheme `{other}`",
                            http_method_title(method)
                        ),
                    ))
                }
            }
        } else {
            parts.port.as_str()
        };
        let request_path = if parts.query.is_empty() {
            parts.path.clone()
        } else {
            format!("{}?{}", parts.path, parts.query)
        };
        let host_header = if parts.port.is_empty() {
            parts.host.clone()
        } else {
            format!("{}:{}", parts.host, parts.port)
        };
        let request = build_http_request(method, &request_path, &host_header, body, content_type);

        match parts.scheme.as_str() {
            "http" => {
                let mut stream = TcpStream::connect(format!("{}:{port}", parts.host))
                    .map_err(|err| RtError::io(err.to_string()))?;
                stream
                    .write_all(request.as_bytes())
                    .map_err(|err| RtError::io(err.to_string()))?;
                read_http_response(stream, method)
            }
            "https" => {
                let tcp = TcpStream::connect(format!("{}:{port}", parts.host))
                    .map_err(|err| RtError::io(err.to_string()))?;
                let server_name = ServerName::try_from(parts.host.clone()).map_err(|_| {
                    RtError::new(
                        RtErrorKind::InvalidArgument,
                        format!(
                            "net.http{} invalid hostname `{}`",
                            http_method_title(method),
                            parts.host
                        ),
                    )
                })?;
                let config = tls_client_config(&self.tls_root_certs)?;
                let mut tls = StreamOwned::new(
                    ClientConnection::new(config, server_name)
                        .map_err(|err| RtError::process(err.to_string()))?,
                    tcp,
                );
                tls.conn
                    .complete_io(&mut tls.sock)
                    .map_err(|err| RtError::io(err.to_string()))?;
                tls.write_all(request.as_bytes())
                    .map_err(|err| RtError::io(err.to_string()))?;
                read_http_response(tls, method)
            }
            _ => unreachable!(),
        }
    }

    fn socket_resource_mut(&mut self, handle: RtHandle) -> RtResult<&mut RtNetResource> {
        self.net_resources.kind_of(handle)?;
        self.net_resources
            .resources
            .get_mut(&handle.id)
            .ok_or_else(|| RtError::invalid_handle(format!("unknown handle id {}", handle.id)))
    }

    fn socket_read(&mut self, handle: RtHandle, buf: &mut [u8]) -> RtResult<usize> {
        match self.socket_resource_mut(handle)? {
            RtNetResource::TcpStream(stream) => stream.read(buf),
            RtNetResource::TlsStream(stream) => stream.read(buf),
            other => {
                return Err(RtError::invalid_handle_kind(
                    RtHandleKind::Socket.type_name(),
                    other.kind().type_name(),
                ))
            }
        }
        .map_err(|err| RtError::io(err.to_string()))
    }

    fn socket_read_exact(&mut self, handle: RtHandle, buf: &mut [u8]) -> RtResult<()> {
        match self.socket_resource_mut(handle)? {
            RtNetResource::TcpStream(stream) => stream.read_exact(buf),
            RtNetResource::TlsStream(stream) => stream.read_exact(buf),
            other => {
                return Err(RtError::invalid_handle_kind(
                    RtHandleKind::Socket.type_name(),
                    other.kind().type_name(),
                ))
            }
        }
        .map_err(|err| RtError::io(err.to_string()))
    }

    fn socket_write_all(&mut self, handle: RtHandle, data: &[u8]) -> RtResult<()> {
        match self.socket_resource_mut(handle)? {
            RtNetResource::TcpStream(stream) => stream.write_all(data),
            RtNetResource::TlsStream(stream) => stream.write_all(data),
            other => {
                return Err(RtError::invalid_handle_kind(
                    RtHandleKind::Socket.type_name(),
                    other.kind().type_name(),
                ))
            }
        }
        .map_err(|err| RtError::io(err.to_string()))
    }

    fn socket_flush(&mut self, handle: RtHandle) -> RtResult<()> {
        match self.socket_resource_mut(handle)? {
            RtNetResource::TcpStream(stream) => stream.flush(),
            RtNetResource::TlsStream(stream) => stream.flush(),
            other => {
                return Err(RtError::invalid_handle_kind(
                    RtHandleKind::Socket.type_name(),
                    other.kind().type_name(),
                ))
            }
        }
        .map_err(|err| RtError::io(err.to_string()))
    }

    fn socket_local_addr(&mut self, handle: RtHandle) -> RtResult<std::net::SocketAddr> {
        match self.socket_resource_mut(handle)? {
            RtNetResource::TcpStream(stream) => stream.local_addr(),
            RtNetResource::TlsStream(stream) => stream.sock.local_addr(),
            other => {
                return Err(RtError::invalid_handle_kind(
                    RtHandleKind::Socket.type_name(),
                    other.kind().type_name(),
                ))
            }
        }
        .map_err(|err| RtError::io(err.to_string()))
    }

    fn socket_peer_addr(&mut self, handle: RtHandle) -> RtResult<std::net::SocketAddr> {
        match self.socket_resource_mut(handle)? {
            RtNetResource::TcpStream(stream) => stream.peer_addr(),
            RtNetResource::TlsStream(stream) => stream.sock.peer_addr(),
            other => {
                return Err(RtError::invalid_handle_kind(
                    RtHandleKind::Socket.type_name(),
                    other.kind().type_name(),
                ))
            }
        }
        .map_err(|err| RtError::io(err.to_string()))
    }

    fn socket_set_read_timeout(
        &mut self,
        handle: RtHandle,
        timeout: Option<Duration>,
    ) -> RtResult<()> {
        match self.socket_resource_mut(handle)? {
            RtNetResource::TcpStream(stream) => stream.set_read_timeout(timeout),
            RtNetResource::TlsStream(stream) => stream.sock.set_read_timeout(timeout),
            other => {
                return Err(RtError::invalid_handle_kind(
                    RtHandleKind::Socket.type_name(),
                    other.kind().type_name(),
                ))
            }
        }
        .map_err(|err| RtError::io(err.to_string()))
    }

    fn socket_set_write_timeout(
        &mut self,
        handle: RtHandle,
        timeout: Option<Duration>,
    ) -> RtResult<()> {
        match self.socket_resource_mut(handle)? {
            RtNetResource::TcpStream(stream) => stream.set_write_timeout(timeout),
            RtNetResource::TlsStream(stream) => stream.sock.set_write_timeout(timeout),
            other => {
                return Err(RtError::invalid_handle_kind(
                    RtHandleKind::Socket.type_name(),
                    other.kind().type_name(),
                ))
            }
        }
        .map_err(|err| RtError::io(err.to_string()))
    }
}

fn duration_from_timeout_millis(name: &str, millis: i64) -> RtResult<Option<Duration>> {
    if millis < 0 {
        return Err(RtError::new(
            RtErrorKind::InvalidArgument,
            format!("{name} millis must be non-negative"),
        ));
    }
    if millis == 0 {
        Ok(None)
    } else {
        Ok(Some(Duration::from_millis(millis as u64)))
    }
}

struct ParsedUrlParts {
    scheme: String,
    host: String,
    port: String,
    path: String,
    query: String,
    fragment: String,
}

fn parse_url_parts(url: &str) -> RtResult<ParsedUrlParts> {
    let (scheme, rest) = url.split_once("://").ok_or_else(|| {
        RtError::new(
            RtErrorKind::InvalidArgument,
            format!("net.parseUrl expected scheme://host in `{url}`"),
        )
    })?;
    if scheme.is_empty() {
        return Err(RtError::new(
            RtErrorKind::InvalidArgument,
            "net.parseUrl URL scheme must not be empty",
        ));
    }

    let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    let mut tail = &rest[authority_end..];
    if authority.is_empty() {
        return Err(RtError::new(
            RtErrorKind::InvalidArgument,
            "net.parseUrl URL host must not be empty",
        ));
    }

    let (host, port) = if authority.starts_with('[') {
        let end = authority.find(']').ok_or_else(|| {
            RtError::new(
                RtErrorKind::InvalidArgument,
                "net.parseUrl invalid IPv6 host syntax",
            )
        })?;
        let host = authority[..=end].to_string();
        let remainder = &authority[end + 1..];
        if remainder.is_empty() {
            (host, String::new())
        } else if let Some(port) = remainder.strip_prefix(':') {
            (host, port.to_string())
        } else {
            return Err(RtError::new(
                RtErrorKind::InvalidArgument,
                "net.parseUrl invalid authority suffix",
            ));
        }
    } else if let Some((host, port)) = authority.rsplit_once(':') {
        if host.is_empty() || port.is_empty() || host.contains(':') {
            (authority.to_string(), String::new())
        } else {
            (host.to_string(), port.to_string())
        }
    } else {
        (authority.to_string(), String::new())
    };

    let mut path = "/".to_string();
    let mut query = String::new();
    let mut fragment = String::new();

    if let Some(fragment_start) = tail.find('#') {
        fragment = tail[fragment_start + 1..].to_string();
        tail = &tail[..fragment_start];
    }
    if let Some(query_start) = tail.find('?') {
        query = tail[query_start + 1..].to_string();
        tail = &tail[..query_start];
    }
    if !tail.is_empty() {
        path = tail.to_string();
    }

    Ok(ParsedUrlParts {
        scheme: scheme.to_string(),
        host,
        port,
        path,
        query,
        fragment,
    })
}

struct HttpResponseParts {
    status: String,
    body: String,
    content_type: String,
}

fn read_http_response(mut reader: impl Read, method: &str) -> RtResult<HttpResponseParts> {
    let mut buf = Vec::new();
    reader
        .read_to_end(&mut buf)
        .map_err(|err| RtError::io(err.to_string()))?;
    let text = String::from_utf8(buf).map_err(|_| {
        RtError::new(
            RtErrorKind::InvalidArgument,
            format!(
                "net.http{} expected valid UTF-8 response data",
                http_method_title(method)
            ),
        )
    })?;
    let (head, body) = text.split_once("\r\n\r\n").ok_or_else(|| {
        RtError::new(
            RtErrorKind::InvalidArgument,
            format!(
                "net.http{} received malformed HTTP response",
                http_method_title(method)
            ),
        )
    })?;
    let mut lines = head.lines();
    let status_line = lines.next().ok_or_else(|| {
        RtError::new(
            RtErrorKind::InvalidArgument,
            format!(
                "net.http{} response missing status line",
                http_method_title(method)
            ),
        )
    })?;
    let status = status_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("")
        .to_string();
    let mut content_type = String::new();
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("content-type") {
                content_type = value.trim().to_string();
                break;
            }
        }
    }
    Ok(HttpResponseParts {
        status,
        body: body.to_string(),
        content_type,
    })
}

fn http_method_title(method: &str) -> &'static str {
    match method {
        "GET" => "Get",
        "POST" => "Post",
        _ => "",
    }
}

fn build_http_request(
    method: &str,
    request_path: &str,
    host_header: &str,
    body: &str,
    content_type: &str,
) -> String {
    if method == "POST" {
        let content_type_header = if content_type.is_empty() {
            String::new()
        } else {
            format!("Content-Type: {content_type}\r\n")
        };
        format!(
            "POST {request_path} HTTP/1.0\r\nHost: {host_header}\r\n{content_type_header}Content-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
    } else {
        format!(
            "{method} {request_path} HTTP/1.0\r\nHost: {host_header}\r\nConnection: close\r\n\r\n"
        )
    }
}

fn tls_client_config(extra_roots: &[CertificateDer<'static>]) -> RtResult<Arc<ClientConfig>> {
    let mut roots = RootCertStore::empty();
    let cert_result = rustls_native_certs::load_native_certs();
    let rustls_native_certs::CertificateResult { certs, errors, .. } = cert_result;
    if certs.is_empty() && errors.is_empty() && extra_roots.is_empty() {
        return Err(RtError::io(
            "no native root CA certificates found".to_string(),
        ));
    }
    let _ = errors;
    for cert in certs {
        roots
            .add(cert)
            .map_err(|err| RtError::process(err.to_string()))?;
    }
    for cert in extra_roots.iter().cloned() {
        roots
            .add(cert)
            .map_err(|err| RtError::process(err.to_string()))?;
    }
    Ok(Arc::new(
        ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth(),
    ))
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
