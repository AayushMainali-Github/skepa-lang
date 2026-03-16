use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{RtError, RtErrorKind, RtResult, RtString};

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

    fn datetime_from_unix(&mut self, _value: i64) -> RtResult<i64> {
        Err(RtError::unsupported_builtin("datetime.fromUnix"))
    }

    fn datetime_from_millis(&mut self, _value: i64) -> RtResult<i64> {
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

    fn os_cwd(&mut self) -> RtResult<RtString> {
        Err(RtError::unsupported_builtin("os.cwd"))
    }

    fn os_platform(&mut self) -> RtResult<RtString> {
        Err(RtError::unsupported_builtin("os.platform"))
    }

    fn os_sleep(&mut self, _millis: i64) -> RtResult<()> {
        Err(RtError::unsupported_builtin("os.sleep"))
    }

    fn os_exec_shell(&mut self, _command: &str) -> RtResult<i64> {
        Err(RtError::unsupported_builtin("os.execShell"))
    }

    fn os_exec_shell_out(&mut self, _command: &str) -> RtResult<RtString> {
        Err(RtError::unsupported_builtin("os.execShellOut"))
    }
}

#[derive(Default)]
pub struct NoopHost;

impl RtHost for NoopHost {
    fn io_print(&mut self, text: &str) -> RtResult<()> {
        print!("{text}");
        std::io::stdout()
            .flush()
            .map_err(|err| RtError::new(RtErrorKind::InvalidArgument, err.to_string()))?;
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
}
