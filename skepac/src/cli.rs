use std::env;

use crate::commands::{
    build_llvm_ir_file, build_native_file, build_object_file, check_file, run_native_file,
};

pub const EXIT_OK: u8 = 0;
pub const EXIT_USAGE: u8 = 2;
pub const EXIT_IO: u8 = 3;
pub const EXIT_PARSE: u8 = 10;
pub const EXIT_SEMA: u8 = 11;
pub const EXIT_CODEGEN: u8 = 12;
pub const EXIT_RESOLVE: u8 = 15;

const USAGE_TOP: &str = "Usage: skepac check <entry.sk> | skepac run <entry.sk> | skepac build-native <entry.sk> <out.exe> | skepac build-obj <entry.sk> <out.obj> | skepac build-llvm-ir <entry.sk> <out.ll>";
const USAGE_CHECK: &str = "Usage: skepac check <file.sk>";
const USAGE_RUN: &str = "Usage: skepac run <in.sk>";
const USAGE_BUILD_NATIVE: &str = "Usage: skepac build-native <in.sk> <out.exe>";
const USAGE_BUILD_OBJ: &str = "Usage: skepac build-obj <in.sk> <out.obj>";
const USAGE_BUILD_LLVM_IR: &str = "Usage: skepac build-llvm-ir <in.sk> <out.ll>";

pub fn run() -> Result<i32, String> {
    let mut args = env::args().skip(1);
    let Some(cmd) = args.next() else {
        return Err(USAGE_TOP.to_string());
    };

    match cmd.as_str() {
        "check" => {
            let Some(path) = args.next() else {
                return Err(USAGE_CHECK.to_string());
            };
            if args.next().is_some() {
                return Err(USAGE_CHECK.to_string());
            }
            check_file(&path)
        }
        "run" => {
            let Some(input) = args.next() else {
                return Err(USAGE_RUN.to_string());
            };
            if args.next().is_some() {
                return Err(USAGE_RUN.to_string());
            }
            run_native_file(&input)
        }
        "build-native" => {
            let Some(input) = args.next() else {
                return Err(USAGE_BUILD_NATIVE.to_string());
            };
            let Some(output) = args.next() else {
                return Err(USAGE_BUILD_NATIVE.to_string());
            };
            if args.next().is_some() {
                return Err(USAGE_BUILD_NATIVE.to_string());
            }
            build_native_file(&input, &output)
        }
        "build-llvm-ir" => {
            let Some(input) = args.next() else {
                return Err(USAGE_BUILD_LLVM_IR.to_string());
            };
            let Some(output) = args.next() else {
                return Err(USAGE_BUILD_LLVM_IR.to_string());
            };
            if args.next().is_some() {
                return Err(USAGE_BUILD_LLVM_IR.to_string());
            }
            build_llvm_ir_file(&input, &output)
        }
        "build-obj" => {
            let Some(input) = args.next() else {
                return Err(USAGE_BUILD_OBJ.to_string());
            };
            let Some(output) = args.next() else {
                return Err(USAGE_BUILD_OBJ.to_string());
            };
            if args.next().is_some() {
                return Err(USAGE_BUILD_OBJ.to_string());
            }
            build_object_file(&input, &output)
        }
        _ => Err(
            "Unknown command. Supported: check, run, build-native, build-obj, build-llvm-ir"
                .to_string(),
        ),
    }
}
