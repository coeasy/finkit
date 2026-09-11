from pathlib import Path

DEP = 'finkit-ffi-common = { path = "../ffi-common" }\n'


def add_dep(path: str, dep: str = DEP) -> None:
    p = Path(path)
    if not p.exists():
        return
    text = p.read_text()
    if 'finkit-ffi-common' in text:
        return
    marker = '[dependencies]\n'
    if marker not in text:
        raise RuntimeError(f'missing dependencies section in {path}')
    p.write_text(text.replace(marker, marker + dep, 1))


def append_once(path: str, marker: str, code: str) -> None:
    p = Path(path)
    if not p.exists():
        raise RuntimeError(f'missing source {path}')
    text = p.read_text()
    if marker not in text:
        p.write_text(text.rstrip() + '\n\n' + code.strip() + '\n')


for cargo in [
    'ffi/node-binding/Cargo.toml',
    'ffi/java-binding/Cargo.toml',
    'ffi/android-binding/Cargo.toml',
    'ffi/dotnet-binding/Cargo.toml',
    'ffi/go-binding/Cargo.toml',
    'ffi/ios-binding/Cargo.toml',
]:
    add_dep(cargo)
add_dep('wasm/Cargo.toml', 'finkit-ffi-common = { path = "../ffi/ffi-common" }\n')

append_once('ffi/node-binding/src/lib.rs', 'pub fn factor_study_json(', r'''
/// Run the versioned Rust factor-research contract from Node.js.
#[napi_derive::napi]
pub fn factor_study_json(request_json: String) -> String {
    finkit_ffi_common::factor_study_json(&request_json)
}
''')

append_once('ffi/java-binding/src/lib.rs', 'Java_com_finkit_FactorResearch_factorStudyJson', r'''
/// JNI bridge for the language-neutral factor research API.
#[no_mangle]
pub extern "system" fn Java_com_finkit_FactorResearch_factorStudyJson(
    mut env: jni::JNIEnv<'_>,
    _class: jni::objects::JClass<'_>,
    request: jni::objects::JString<'_>,
) -> jni::sys::jstring {
    let request: String = match env.get_string(&request) {
        Ok(value) => value.into(),
        Err(error) => {
            let fallback = finkit_ffi_common::factor_study_error_json(
                "invalid_utf8",
                &error.to_string(),
            );
            return env
                .new_string(fallback)
                .map(|value| value.into_raw())
                .unwrap_or(std::ptr::null_mut());
        }
    };
    env.new_string(finkit_ffi_common::factor_study_json(&request))
        .map(|value| value.into_raw())
        .unwrap_or(std::ptr::null_mut())
}
''')
java = Path('ffi/java-binding/java/src/main/java/com/finkit/FactorResearch.java')
java.parent.mkdir(parents=True, exist_ok=True)
java.write_text('''package com.finkit;\n\n/** Versioned panel-aware factor research backed by the canonical Rust engine. */\npublic final class FactorResearch {\n    static { NativeLoader.load(); }\n    private FactorResearch() {}\n    public static native String factorStudyJson(String requestJson);\n}\n''')

append_once('ffi/android-binding/src/lib.rs', 'Java_com_finkit_indicators_Finkit_factorStudyJson', r'''
/// Android JNI bridge for the canonical factor research JSON contract.
#[no_mangle]
pub extern "system" fn Java_com_finkit_indicators_Finkit_factorStudyJson(
    mut env: jni::JNIEnv<'_>,
    _class: jni::objects::JClass<'_>,
    request: jni::objects::JString<'_>,
) -> jni::sys::jstring {
    let response = match env.get_string(&request) {
        Ok(value) => {
            let request: String = value.into();
            finkit_ffi_common::factor_study_json(&request)
        }
        Err(error) => finkit_ffi_common::factor_study_error_json(
            "invalid_utf8",
            &error.to_string(),
        ),
    };
    env.new_string(response)
        .map(|value| value.into_raw())
        .unwrap_or(std::ptr::null_mut())
}
''')
android = Path('ffi/android-binding/android/src/main/java/com/finkit/indicators/Finkit.java')
text = android.read_text()
decl = '    public static native String factorStudyJson(String requestJson);\n'
if decl not in text:
    end = text.rfind('}')
    text = text[:end].rstrip() + '\n\n    /** Run schema-versioned panel factor research and return its JSON envelope. */\n' + decl + text[end:]
    android.write_text(text)

NATIVE_HELPERS = r'''
fn research_string_ptr(value: String) -> *mut std::os::raw::c_char {
    std::ffi::CString::new(value)
        .map(std::ffi::CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}
unsafe fn research_request_json(ptr: *const std::os::raw::c_char) -> String {
    if ptr.is_null() {
        return finkit_ffi_common::factor_study_error_json(
            "null_pointer",
            "request_json is null",
        );
    }
    let request = unsafe { std::ffi::CStr::from_ptr(ptr) };
    match request.to_str() {
        Ok(request) => finkit_ffi_common::factor_study_json(request),
        Err(error) => finkit_ffi_common::factor_study_error_json(
            "invalid_utf8",
            &error.to_string(),
        ),
    }
}
'''

append_once('ffi/dotnet-binding/src/lib.rs', 'finkit_dotnet_factor_study_json', NATIVE_HELPERS + r'''
#[no_mangle]
pub unsafe extern "C" fn finkit_dotnet_factor_study_json(
    request_json: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    research_string_ptr(unsafe { research_request_json(request_json) })
}
#[no_mangle]
pub unsafe extern "C" fn finkit_dotnet_factor_study_free_string(value: *mut std::os::raw::c_char) {
    if !value.is_null() {
        drop(unsafe { std::ffi::CString::from_raw(value) });
    }
}
''')
Path('ffi/dotnet-binding/src/Finkit/FactorResearch.cs').write_text(r'''using System;
using System.Runtime.InteropServices;
namespace Finkit;
/// <summary>Versioned panel factor research backed by the canonical Rust engine.</summary>
public static class FactorResearch
{
    private const string NativeLibrary = "finkit_dotnet";
    [DllImport(NativeLibrary, CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr finkit_dotnet_factor_study_json([MarshalAs(UnmanagedType.LPUTF8Str)] string requestJson);
    [DllImport(NativeLibrary, CallingConvention = CallingConvention.Cdecl)]
    private static extern void finkit_dotnet_factor_study_free_string(IntPtr value);
    public static string RunJson(string requestJson)
    {
        ArgumentNullException.ThrowIfNull(requestJson);
        var ptr = finkit_dotnet_factor_study_json(requestJson);
        if (ptr == IntPtr.Zero) throw new InvalidOperationException("Native factor research returned null");
        try { return Marshal.PtrToStringUTF8(ptr) ?? string.Empty; }
        finally { finkit_dotnet_factor_study_free_string(ptr); }
    }
}
''')

append_once('ffi/go-binding/src/lib.rs', 'finkit_go_factor_study_json', NATIVE_HELPERS + r'''
#[no_mangle]
pub unsafe extern "C" fn finkit_go_factor_study_json(
    request_json: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    research_string_ptr(unsafe { research_request_json(request_json) })
}
#[no_mangle]
pub unsafe extern "C" fn finkit_go_factor_study_free_string(value: *mut std::os::raw::c_char) {
    if !value.is_null() {
        drop(unsafe { std::ffi::CString::from_raw(value) });
    }
}
''')
Path('ffi/go-binding/go/ta/research.go').write_text(r'''package ta
/*
#cgo LDFLAGS: -lfinkit_go
#include <stdlib.h>
char* finkit_go_factor_study_json(const char* request_json);
void finkit_go_factor_study_free_string(char* value);
*/
import "C"
import (
    "errors"
    "unsafe"
)
// FactorStudyJSON runs the schema-versioned canonical Rust factor research engine.
func FactorStudyJSON(requestJSON string) (string, error) {
    request := C.CString(requestJSON)
    defer C.free(unsafe.Pointer(request))
    response := C.finkit_go_factor_study_json(request)
    if response == nil { return "", errors.New("native factor research returned nil") }
    defer C.finkit_go_factor_study_free_string(response)
    return C.GoString(response), nil
}
''')

append_once('ffi/ios-binding/src/lib.rs', 'finkit_ios_factor_study_json', NATIVE_HELPERS + r'''
#[no_mangle]
pub unsafe extern "C" fn finkit_ios_factor_study_json(
    request_json: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    research_string_ptr(unsafe { research_request_json(request_json) })
}
#[no_mangle]
pub unsafe extern "C" fn finkit_ios_factor_study_free_string(value: *mut std::os::raw::c_char) {
    if !value.is_null() {
        drop(unsafe { std::ffi::CString::from_raw(value) });
    }
}
''')
swift_files = list(Path('ffi/ios-binding').glob('**/*.swift'))
if swift_files:
    (swift_files[0].parent / 'FactorResearch.swift').write_text(r'''import Foundation
@_silgen_name("finkit_ios_factor_study_json")
private func nativeFactorStudyJSON(_ request: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar>?
@_silgen_name("finkit_ios_factor_study_free_string")
private func nativeFactorStudyFreeString(_ value: UnsafeMutablePointer<CChar>)
public enum FactorResearch {
    public static func runJSON(_ request: String) throws -> String {
        guard let output = request.withCString({ nativeFactorStudyJSON($0) }) else {
            throw NSError(domain: "Finkit.FactorResearch", code: 1, userInfo: [NSLocalizedDescriptionKey: "Native factor research returned nil"])
        }
        defer { nativeFactorStudyFreeString(output) }
        return String(cString: output)
    }
}
''')

append_once('wasm/src/lib.rs', 'pub fn factor_study_json(', r'''
/// Execute the same schema-versioned factor research contract used by native bindings.
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn factor_study_json(request_json: &str) -> String {
    finkit_ffi_common::factor_study_json(request_json)
}
''')
