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

macro_rules! default_builtins {
    ($(($id:expr, $pkg:literal, $name:literal, $handler:path)),+ $(,)?) => {
        pub(crate) fn default_builtin_name_by_id(id: u16) -> Option<&'static str> {
            match id {
                $($id => Some(concat!($pkg, ".", $name)),)+
                _ => None,
            }
        }

        pub(crate) fn default_builtin_id(package: &str, name: &str) -> Option<u16> {
            match (package, name) {
                $(($pkg, $name) => Some($id),)+
                _ => None,
            }
        }

        pub(crate) fn call_default_builtin_by_id(
            host: &mut dyn BuiltinHost,
            id: u16,
            args: Vec<Value>,
        ) -> Result<Value, VmError> {
            match id {
                $($id => $handler(host, args),)+
                _ => Err(VmError::new(
                    VmErrorKind::UnknownBuiltin,
                    format!("Unknown builtin id `{id}`"),
                )),
            }
        }

        fn default_handler(package: &str, name: &str) -> Option<BuiltinHandler> {
            match (package, name) {
                $(($pkg, $name) => Some($handler),)+
                _ => None,
            }
        }
    };
}

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

    pub(crate) fn call_by_id(
        &self,
        host: &mut dyn BuiltinHost,
        id: u16,
        args: Vec<Value>,
    ) -> Result<Value, VmError> {
        call_default_builtin_by_id(host, id, args)
    }
}

default_builtins!(
    (0, "io", "print", io::builtin_io_print),
    (1, "io", "println", io::builtin_io_println),
    (2, "io", "printInt", io::builtin_io_print_int),
    (3, "io", "printFloat", io::builtin_io_print_float),
    (4, "io", "printBool", io::builtin_io_print_bool),
    (5, "io", "printString", io::builtin_io_print_string),
    (6, "io", "format", io::builtin_io_format),
    (7, "io", "printf", io::builtin_io_printf),
    (8, "io", "readLine", io::builtin_io_readline),
    (9, "str", "len", str::builtin_str_len),
    (10, "str", "contains", str::builtin_str_contains),
    (11, "str", "startsWith", str::builtin_str_starts_with),
    (12, "str", "endsWith", str::builtin_str_ends_with),
    (13, "str", "trim", str::builtin_str_trim),
    (14, "str", "toLower", str::builtin_str_to_lower),
    (15, "str", "toUpper", str::builtin_str_to_upper),
    (16, "str", "indexOf", str::builtin_str_index_of),
    (17, "str", "slice", str::builtin_str_slice),
    (18, "str", "isEmpty", str::builtin_str_is_empty),
    (19, "str", "lastIndexOf", str::builtin_str_last_index_of),
    (20, "str", "replace", str::builtin_str_replace),
    (21, "str", "repeat", str::builtin_str_repeat),
    (22, "arr", "len", arr::builtin_arr_len),
    (23, "arr", "isEmpty", arr::builtin_arr_is_empty),
    (24, "arr", "contains", arr::builtin_arr_contains),
    (25, "arr", "indexOf", arr::builtin_arr_index_of),
    (26, "arr", "count", arr::builtin_arr_count),
    (27, "arr", "first", arr::builtin_arr_first),
    (28, "arr", "last", arr::builtin_arr_last),
    (29, "arr", "join", arr::builtin_arr_join),
    (
        30,
        "datetime",
        "nowUnix",
        datetime::builtin_datetime_now_unix
    ),
    (
        31,
        "datetime",
        "nowMillis",
        datetime::builtin_datetime_now_millis
    ),
    (
        32,
        "datetime",
        "fromUnix",
        datetime::builtin_datetime_from_unix
    ),
    (
        33,
        "datetime",
        "fromMillis",
        datetime::builtin_datetime_from_millis
    ),
    (
        34,
        "datetime",
        "parseUnix",
        datetime::builtin_datetime_parse_unix
    ),
    (35, "datetime", "year", datetime::builtin_datetime_year),
    (36, "datetime", "month", datetime::builtin_datetime_month),
    (37, "datetime", "day", datetime::builtin_datetime_day),
    (38, "datetime", "hour", datetime::builtin_datetime_hour),
    (39, "datetime", "minute", datetime::builtin_datetime_minute),
    (40, "datetime", "second", datetime::builtin_datetime_second),
    (41, "fs", "exists", fs::builtin_fs_exists),
    (42, "fs", "readText", fs::builtin_fs_read_text),
    (43, "fs", "writeText", fs::builtin_fs_write_text),
    (44, "fs", "appendText", fs::builtin_fs_append_text),
    (45, "fs", "mkdirAll", fs::builtin_fs_mkdir_all),
    (46, "fs", "removeFile", fs::builtin_fs_remove_file),
    (47, "fs", "removeDirAll", fs::builtin_fs_remove_dir_all),
    (48, "fs", "join", fs::builtin_fs_join),
    (49, "os", "cwd", os::builtin_os_cwd),
    (50, "os", "platform", os::builtin_os_platform),
    (51, "os", "sleep", os::builtin_os_sleep),
    (52, "os", "execShell", os::builtin_os_exec_shell),
    (53, "os", "execShellOut", os::builtin_os_exec_shell_out),
    (54, "random", "seed", random::builtin_random_seed),
    (55, "random", "int", random::builtin_random_int),
    (56, "random", "float", random::builtin_random_float),
    (57, "vec", "new", vec::builtin_vec_new),
    (58, "vec", "len", vec::builtin_vec_len),
    (59, "vec", "push", vec::builtin_vec_push),
    (60, "vec", "get", vec::builtin_vec_get),
    (61, "vec", "set", vec::builtin_vec_set),
    (62, "vec", "delete", vec::builtin_vec_delete),
);
