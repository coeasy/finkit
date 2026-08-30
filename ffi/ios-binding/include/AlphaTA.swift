// Swift wrapper around the AlphaTA C ABI.
//
// The .xcframework ships both the static library and this source file; the
// consumer's Xcode project just adds `import AlphaTA` after dragging the
// .xcframework into the project.

import Foundation

public enum AlphaTAError: Error, CustomStringConvertible {
    case abiMismatch(expected: Int, got: Int)
    case invalidParameters
    case calculationFailed

    public var description: String {
        switch self {
        case .abiMismatch(let e, let g):
            return "AlphaTA ABI mismatch: framework=\(e) binary=\(g)"
        case .invalidParameters:
            return "AlphaTA: invalid parameters"
        case .calculationFailed:
            return "AlphaTA: calculation failed"
        }
    }
}

public enum AlphaTA {
    /// ABI version of the bundled static library.
    public static let abiVersion: Int32 = alpha_ta_ios_abi_version()

    // ---- moving averages ----
    public static func sma(_ input: [Double], period: Int32) throws -> [Double] {
        var out = [Double](repeating: 0, count: input.count)
        let rc = input.withUnsafeBufferPointer { inp in
            out.withUnsafeMutableBufferPointer { oinp in
                alpha_ta_sma(inp.baseAddress, Int32(input.count), period, oinp.baseAddress)
            }
        }
        if rc == 0 { return out } throw AlphaTAError.calculationFailed
    }

    public static func ema(_ input: [Double], period: Int32) throws -> [Double] {
        var out = [Double](repeating: 0, count: input.count)
        let rc = input.withUnsafeBufferPointer { inp in
            out.withUnsafeMutableBufferPointer { oinp in
                alpha_ta_ema(inp.baseAddress, Int32(input.count), period, oinp.baseAddress)
            }
        }
        if rc == 0 { return out } throw AlphaTAError.calculationFailed
    }

    public static func wma(_ input: [Double], period: Int32) throws -> [Double] {
        var out = [Double](repeating: 0, count: input.count)
        let rc = input.withUnsafeBufferPointer { inp in
            out.withUnsafeMutableBufferPointer { oinp in
                alpha_ta_wma(inp.baseAddress, Int32(input.count), period, oinp.baseAddress)
            }
        }
        if rc == 0 { return out } throw AlphaTAError.calculationFailed
    }

    // ---- momentum ----
    public static func rsi(_ input: [Double], period: Int32) throws -> [Double] {
        var out = [Double](repeating: 0, count: input.count)
        let rc = input.withUnsafeBufferPointer { inp in
            out.withUnsafeMutableBufferPointer { oinp in
                alpha_ta_rsi(inp.baseAddress, Int32(input.count), period, oinp.baseAddress)
            }
        }
        if rc == 0 { return out } throw AlphaTAError.calculationFailed
    }

    public static func roc(_ input: [Double], period: Int32) throws -> [Double] {
        var out = [Double](repeating: 0, count: input.count)
        let rc = input.withUnsafeBufferPointer { inp in
            out.withUnsafeMutableBufferPointer { oinp in
                alpha_ta_roc(inp.baseAddress, Int32(input.count), period, oinp.baseAddress)
            }
        }
        if rc == 0 { return out } throw AlphaTAError.calculationFailed
    }

    public static func mom(_ input: [Double], period: Int32) throws -> [Double] {
        var out = [Double](repeating: 0, count: input.count)
        let rc = input.withUnsafeBufferPointer { inp in
            out.withUnsafeMutableBufferPointer { oinp in
                alpha_ta_mom(inp.baseAddress, Int32(input.count), period, oinp.baseAddress)
            }
        }
        if rc == 0 { return out } throw AlphaTAError.calculationFailed
    }
}