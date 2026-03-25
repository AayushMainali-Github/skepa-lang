#![allow(dead_code)]

use std::collections::HashMap;

use skepart::{RtHost, RtResult, RtString};

#[derive(Default)]
pub struct RecordingHost {
    pub output: String,
    pub unix_now: i64,
    pub millis_now: i64,
    pub random_int_value: i64,
    pub random_float_value: f64,
    pub cwd: String,
    pub platform: String,
    pub arch: String,
    pub args: Vec<String>,
    pub read_line: String,
    pub shell_status: i64,
    pub shell_out: String,
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
            cwd: "tmp/work".into(),
            platform: "test-os".into(),
            arch: "test-arch".into(),
            args: vec!["skepa".into(), "--flag".into()],
            read_line: "typed line".into(),
            shell_status: 9,
            shell_out: "shell-out".into(),
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

    pub fn cwd(mut self, value: impl Into<String>) -> Self {
        self.host.cwd = value.into();
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

    pub fn shell_status(mut self, value: i64) -> Self {
        self.host.shell_status = value;
        self
    }

    pub fn shell_out(mut self, value: impl Into<String>) -> Self {
        self.host.shell_out = value.into();
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

    fn os_cwd(&mut self) -> RtResult<RtString> {
        Ok(RtString::from(self.cwd.clone()))
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

    fn os_exec(&mut self, program: &str) -> RtResult<i64> {
        self.output.push_str(&format!("[exec {program}]"));
        Ok(self.shell_status)
    }

    fn os_exec_out(&mut self, program: &str) -> RtResult<RtString> {
        self.output.push_str(&format!("[execout {program}]"));
        Ok(RtString::from(self.shell_out.clone()))
    }

    fn os_exec_shell(&mut self, command: &str) -> RtResult<i64> {
        self.output.push_str(&format!("[sh {command}]"));
        Ok(self.shell_status)
    }

    fn os_exec_shell_out(&mut self, command: &str) -> RtResult<RtString> {
        self.output.push_str(&format!("[shout {command}]"));
        Ok(RtString::from(self.shell_out.clone()))
    }
}
