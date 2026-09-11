from pathlib import Path


def replace_once(path: str, needle: str, replacement: str) -> None:
    p = Path(path)
    text = p.read_text()
    if replacement in text:
        return
    if needle not in text:
        raise RuntimeError(f'missing integration anchor in {path}: {needle!r}')
    p.write_text(text.replace(needle, replacement, 1))


def append_once(path: str, marker: str, code: str) -> None:
    p = Path(path)
    text = p.read_text()
    if marker in text:
        return
    p.write_text(text.rstrip() + '\n\n' + code.strip() + '\n')


# Python: the implementation lives in a small module so the large generated-ish
# root remains stable.
replace_once(
    'ffi/python-binding/src/lib.rs',
    'mod streaming;\n',
    'mod research_api;\nmod streaming;\n',
)
replace_once(
    'ffi/python-binding/src/lib.rs',
    '    // Streaming Indicators\n',
    '    // Factor research and generic quantitative evaluation\n'
    '    research_api::register_research_api(m)?;\n\n'
    '    // Streaming Indicators\n',
)

# Node / N-API.
append_once('ffi/node-binding/src/lib.rs', 'pub fn quant_evaluation_json(', r'''
/// Evaluate strategy/benchmark/trade/portfolio metrics through the canonical Rust engine.
#[napi_derive::napi]
pub fn quant_evaluation_json(request_json: String) -> String {
    finkit_ffi_common::quant_evaluation_json(&request_json)
}
''')

# Desktop Java/JNI.
append_once('ffi/java-binding/src/lib.rs', 'Java_com_finkit_QuantEvaluation_evaluateJson', r'''
#[no_mangle]
pub extern "system" fn Java_com_finkit_QuantEvaluation_evaluateJson(
    mut env: jni::JNIEnv<'_>,
    _class: jni::objects::JClass<'_>,
    request: jni::objects::JString<'_>,
) -> jni::sys::jstring {
    let response = match env.get_string(&request) {
        Ok(value) => {
            let request: String = value.into();
            finkit_ffi_common::quant_evaluation_json(&request)
        }
        Err(error) => finkit_ffi_common::quant_evaluation_error_json(
            "invalid_utf8",
            &error.to_string(),
        ),
    };
    env.new_string(response)
        .map(|value| value.into_raw())
        .unwrap_or(std::ptr::null_mut())
}
''')
java = Path('ffi/java-binding/java/src/main/java/com/finkit/QuantEvaluation.java')
java.parent.mkdir(parents=True, exist_ok=True)
java.write_text('''package com.finkit;\n\n/** Canonical strategy, benchmark, trade and portfolio evaluation. */\npublic final class QuantEvaluation {\n    static { NativeLoader.load(); }\n    private QuantEvaluation() {}\n    public static native String evaluateJson(String requestJson);\n}\n''')

# Android/JNI uses the existing Finkit class.
append_once('ffi/android-binding/src/lib.rs', 'Java_com_finkit_indicators_Finkit_quantEvaluationJson', r'''
#[no_mangle]
pub extern "system" fn Java_com_finkit_indicators_Finkit_quantEvaluationJson(
    mut env: jni::JNIEnv<'_>,
    _class: jni::objects::JClass<'_>,
    request: jni::objects::JString<'_>,
) -> jni::sys::jstring {
    let response = match env.get_string(&request) {
        Ok(value) => {
            let request: String = value.into();
            finkit_ffi_common::quant_evaluation_json(&request)
        }
        Err(error) => finkit_ffi_common::quant_evaluation_error_json(
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
decl = '    public static native String quantEvaluationJson(String requestJson);\n'
if decl not in text:
    end = text.rfind('}')
    text = text[:end].rstrip() + '\n\n    /** Evaluate returns, risk, benchmark, trade, portfolio and costs. */\n' + decl + text[end:]
    android.write_text(text)

# Native pointer bridges for .NET / Go / iOS already have common request helpers
# after research_multilang_integrate.py runs. Reuse those helpers.
append_once('ffi/dotnet-binding/src/lib.rs', 'finkit_dotnet_quant_evaluation_json', r'''
#[no_mangle]
pub unsafe extern "C" fn finkit_dotnet_quant_evaluation_json(
    request_json: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    if request_json.is_null() {
        return research_string_ptr(finkit_ffi_common::quant_evaluation_error_json(
            "null_pointer",
            "request_json is null",
        ));
    }
    let request = unsafe { std::ffi::CStr::from_ptr(request_json) };
    let response = match request.to_str() {
        Ok(value) => finkit_ffi_common::quant_evaluation_json(value),
        Err(error) => finkit_ffi_common::quant_evaluation_error_json(
            "invalid_utf8",
            &error.to_string(),
        ),
    };
    research_string_ptr(response)
}
''')
Path('ffi/dotnet-binding/src/Finkit/QuantEvaluation.cs').write_text(r'''using System;
using System.Runtime.InteropServices;
namespace Finkit;
/// <summary>Canonical quantitative evaluation backed by Rust.</summary>
public static class QuantEvaluation
{
    private const string NativeLibrary = "finkit_dotnet";
    [DllImport(NativeLibrary, CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr finkit_dotnet_quant_evaluation_json([MarshalAs(UnmanagedType.LPUTF8Str)] string requestJson);
    [DllImport(NativeLibrary, CallingConvention = CallingConvention.Cdecl)]
    private static extern void finkit_dotnet_factor_study_free_string(IntPtr value);
    public static string RunJson(string requestJson)
    {
        ArgumentNullException.ThrowIfNull(requestJson);
        var ptr = finkit_dotnet_quant_evaluation_json(requestJson);
        if (ptr == IntPtr.Zero) throw new InvalidOperationException("Native quantitative evaluation returned null");
        try { return Marshal.PtrToStringUTF8(ptr) ?? string.Empty; }
        finally { finkit_dotnet_factor_study_free_string(ptr); }
    }
}
''')

append_once('ffi/go-binding/src/lib.rs', 'finkit_go_quant_evaluation_json', r'''
#[no_mangle]
pub unsafe extern "C" fn finkit_go_quant_evaluation_json(
    request_json: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    if request_json.is_null() {
        return research_string_ptr(finkit_ffi_common::quant_evaluation_error_json(
            "null_pointer",
            "request_json is null",
        ));
    }
    let request = unsafe { std::ffi::CStr::from_ptr(request_json) };
    let response = match request.to_str() {
        Ok(value) => finkit_ffi_common::quant_evaluation_json(value),
        Err(error) => finkit_ffi_common::quant_evaluation_error_json(
            "invalid_utf8",
            &error.to_string(),
        ),
    };
    research_string_ptr(response)
}
''')
Path('ffi/go-binding/go/ta/quant_evaluation.go').write_text(r'''package ta
/*
#cgo LDFLAGS: -lfinkit_go
#include <stdlib.h>
char* finkit_go_quant_evaluation_json(const char* request_json);
void finkit_go_factor_study_free_string(char* value);
*/
import "C"
import (
    "errors"
    "unsafe"
)
// QuantEvaluationJSON evaluates strategy, benchmark, trade and portfolio metrics.
func QuantEvaluationJSON(requestJSON string) (string, error) {
    request := C.CString(requestJSON)
    defer C.free(unsafe.Pointer(request))
    response := C.finkit_go_quant_evaluation_json(request)
    if response == nil { return "", errors.New("native quantitative evaluation returned nil") }
    defer C.finkit_go_factor_study_free_string(response)
    return C.GoString(response), nil
}
''')

append_once('ffi/ios-binding/src/lib.rs', 'finkit_ios_quant_evaluation_json', r'''
#[no_mangle]
pub unsafe extern "C" fn finkit_ios_quant_evaluation_json(
    request_json: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    if request_json.is_null() {
        return research_string_ptr(finkit_ffi_common::quant_evaluation_error_json(
            "null_pointer",
            "request_json is null",
        ));
    }
    let request = unsafe { std::ffi::CStr::from_ptr(request_json) };
    let response = match request.to_str() {
        Ok(value) => finkit_ffi_common::quant_evaluation_json(value),
        Err(error) => finkit_ffi_common::quant_evaluation_error_json(
            "invalid_utf8",
            &error.to_string(),
        ),
    };
    research_string_ptr(response)
}
''')
swift_files = list(Path('ffi/ios-binding').glob('**/*.swift'))
if swift_files:
    (swift_files[0].parent / 'QuantEvaluation.swift').write_text(r'''import Foundation
@_silgen_name("finkit_ios_quant_evaluation_json")
private func nativeQuantEvaluationJSON(_ request: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar>?
@_silgen_name("finkit_ios_factor_study_free_string")
private func nativeResearchFreeString(_ value: UnsafeMutablePointer<CChar>)
public enum QuantEvaluation {
    public static func runJSON(_ request: String) throws -> String {
        guard let output = request.withCString({ nativeQuantEvaluationJSON($0) }) else {
            throw NSError(domain: "Finkit.QuantEvaluation", code: 1, userInfo: [NSLocalizedDescriptionKey: "Native quantitative evaluation returned nil"])
        }
        defer { nativeResearchFreeString(output) }
        return String(cString: output)
    }
}
''')

append_once('wasm/src/lib.rs', 'pub fn quant_evaluation_json(', r'''
/// Evaluate arbitrary returns through the canonical quantitative evaluation contract.
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn quant_evaluation_json(request_json: &str) -> String {
    finkit_ffi_common::quant_evaluation_json(request_json)
}
''')

# Serde-default the evaluation config container so schema-v2 callers can send
# only the fields they want to override while missing fields keep stable defaults.
performance = Path('factor-analysis/src/performance.rs')
text = performance.read_text()
old = '#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]\npub struct EvaluationConfig {'
new = '#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]\n#[serde(default)]\npub struct EvaluationConfig {'
if old in text and new not in text:
    performance.write_text(text.replace(old, new, 1))
