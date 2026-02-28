mod arr;
mod datetime;
mod fs;
mod io;
mod os;
mod random;
mod str;
mod vec;

use std::collections::HashMap;

use crate::bytecode::Value;

use super::{BuiltinHost, VmError, VmErrorKind};

pub type BuiltinHandler = fn(&mut dyn BuiltinHost, Vec<Value>) -> Result<Value, VmError>;

#[derive(Default)]
pub struct BuiltinRegistry {
    custom_handlers: HashMap<String, HashMap<String, BuiltinHandler>>,
}

impl BuiltinRegistry {
    pub fn with_defaults() -> Self {
        Self::default()
    }

    pub fn register(&mut self, package: &str, name: &str, handler: BuiltinHandler) {
        self.custom_handlers
            .entry(package.to_string())
            .or_default()
            .insert(name.to_string(), handler);
    }

    pub(crate) fn call(
        &self,
        host: &mut dyn BuiltinHost,
        package: &str,
        name: &str,
        args: Vec<Value>,
    ) -> Result<Value, VmError> {
        if let Some(handler) = default_handler(package, name) {
            return handler(host, args);
        }

        let Some(handler) = self
            .custom_handlers
            .get(package)
            .and_then(|pkg| pkg.get(name))
            .copied()
        else {
            return Err(VmError::new(
                VmErrorKind::UnknownBuiltin,
                format!("Unknown builtin `{package}.{name}`"),
            ));
        };
        handler(host, args)
    }
}

fn default_handler(package: &str, name: &str) -> Option<BuiltinHandler> {
    match (package, name) {
        ("io", "print") => Some(io::builtin_io_print),
        ("io", "println") => Some(io::builtin_io_println),
        ("io", "printInt") => Some(io::builtin_io_print_int),
        ("io", "printFloat") => Some(io::builtin_io_print_float),
        ("io", "printBool") => Some(io::builtin_io_print_bool),
        ("io", "printString") => Some(io::builtin_io_print_string),
        ("io", "format") => Some(io::builtin_io_format),
        ("io", "printf") => Some(io::builtin_io_printf),
        ("io", "readLine") => Some(io::builtin_io_readline),
        ("str", "len") => Some(str::builtin_str_len),
        ("str", "contains") => Some(str::builtin_str_contains),
        ("str", "startsWith") => Some(str::builtin_str_starts_with),
        ("str", "endsWith") => Some(str::builtin_str_ends_with),
        ("str", "trim") => Some(str::builtin_str_trim),
        ("str", "toLower") => Some(str::builtin_str_to_lower),
        ("str", "toUpper") => Some(str::builtin_str_to_upper),
        ("str", "indexOf") => Some(str::builtin_str_index_of),
        ("str", "slice") => Some(str::builtin_str_slice),
        ("str", "isEmpty") => Some(str::builtin_str_is_empty),
        ("str", "lastIndexOf") => Some(str::builtin_str_last_index_of),
        ("str", "replace") => Some(str::builtin_str_replace),
        ("str", "repeat") => Some(str::builtin_str_repeat),
        ("arr", "len") => Some(arr::builtin_arr_len),
        ("arr", "isEmpty") => Some(arr::builtin_arr_is_empty),
        ("arr", "contains") => Some(arr::builtin_arr_contains),
        ("arr", "indexOf") => Some(arr::builtin_arr_index_of),
        ("arr", "count") => Some(arr::builtin_arr_count),
        ("arr", "first") => Some(arr::builtin_arr_first),
        ("arr", "last") => Some(arr::builtin_arr_last),
        ("arr", "join") => Some(arr::builtin_arr_join),
        ("datetime", "nowUnix") => Some(datetime::builtin_datetime_now_unix),
        ("datetime", "nowMillis") => Some(datetime::builtin_datetime_now_millis),
        ("datetime", "fromUnix") => Some(datetime::builtin_datetime_from_unix),
        ("datetime", "fromMillis") => Some(datetime::builtin_datetime_from_millis),
        ("datetime", "parseUnix") => Some(datetime::builtin_datetime_parse_unix),
        ("datetime", "year") => Some(datetime::builtin_datetime_year),
        ("datetime", "month") => Some(datetime::builtin_datetime_month),
        ("datetime", "day") => Some(datetime::builtin_datetime_day),
        ("datetime", "hour") => Some(datetime::builtin_datetime_hour),
        ("datetime", "minute") => Some(datetime::builtin_datetime_minute),
        ("datetime", "second") => Some(datetime::builtin_datetime_second),
        ("fs", "exists") => Some(fs::builtin_fs_exists),
        ("fs", "readText") => Some(fs::builtin_fs_read_text),
        ("fs", "writeText") => Some(fs::builtin_fs_write_text),
        ("fs", "appendText") => Some(fs::builtin_fs_append_text),
        ("fs", "mkdirAll") => Some(fs::builtin_fs_mkdir_all),
        ("fs", "removeFile") => Some(fs::builtin_fs_remove_file),
        ("fs", "removeDirAll") => Some(fs::builtin_fs_remove_dir_all),
        ("fs", "join") => Some(fs::builtin_fs_join),
        ("os", "cwd") => Some(os::builtin_os_cwd),
        ("os", "platform") => Some(os::builtin_os_platform),
        ("os", "sleep") => Some(os::builtin_os_sleep),
        ("os", "execShell") => Some(os::builtin_os_exec_shell),
        ("os", "execShellOut") => Some(os::builtin_os_exec_shell_out),
        ("random", "seed") => Some(random::builtin_random_seed),
        ("random", "int") => Some(random::builtin_random_int),
        ("random", "float") => Some(random::builtin_random_float),
        ("vec", "new") => Some(vec::builtin_vec_new),
        ("vec", "len") => Some(vec::builtin_vec_len),
        ("vec", "push") => Some(vec::builtin_vec_push),
        ("vec", "get") => Some(vec::builtin_vec_get),
        ("vec", "set") => Some(vec::builtin_vec_set),
        ("vec", "delete") => Some(vec::builtin_vec_delete),
        _ => None,
    }
}
