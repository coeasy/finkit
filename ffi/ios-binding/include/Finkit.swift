// Swift wrapper around the Finkit iOS C ABI.
//
// The underlying C symbols retain their historical alpha_ta_* prefix for ABI
// compatibility. New source code should use the Finkit/FinkitError names.

import Foundation
import FinkitC

public enum FinkitError: Error, CustomStringConvertible {
    case abiMismatch(expected: Int, got: Int)
    case invalidParameters
    case calculationFailed

    public var description: String {
        switch self {
        case .abiMismatch(let expected, let got):
            return "Finkit ABI mismatch: framework=\(expected) binary=\(got)"
        case .invalidParameters:
            return "Finkit: invalid parameters"
        case .calculationFailed:
            return "Finkit: calculation failed"
        }
    }
}

public enum Finkit {
    /// ABI version of the bundled static library.
    public static let abiVersion: Int32 = alpha_ta_ios_abi_version()

    public static func sma(_ input: [Double], period: Int32) throws -> [Double] {
        var out = [Double](repeating: 0, count: input.count)
        let rc = input.withUnsafeBufferPointer { inp in
            out.withUnsafeMutableBufferPointer { dst in
                alpha_ta_sma(inp.baseAddress, Int32(input.count), period, dst.baseAddress)
            }
        }
        guard rc == 0 else { throw FinkitError.calculationFailed }
        return out
    }

    public static func ema(_ input: [Double], period: Int32) throws -> [Double] {
        var out = [Double](repeating: 0, count: input.count)
        let rc = input.withUnsafeBufferPointer { inp in
            out.withUnsafeMutableBufferPointer { dst in
                alpha_ta_ema(inp.baseAddress, Int32(input.count), period, dst.baseAddress)
            }
        }
        guard rc == 0 else { throw FinkitError.calculationFailed }
        return out
    }

    public static func wma(_ input: [Double], period: Int32) throws -> [Double] {
        var out = [Double](repeating: 0, count: input.count)
        let rc = input.withUnsafeBufferPointer { inp in
            out.withUnsafeMutableBufferPointer { dst in
                alpha_ta_wma(inp.baseAddress, Int32(input.count), period, dst.baseAddress)
            }
        }
        guard rc == 0 else { throw FinkitError.calculationFailed }
        return out
    }

    public static func dema(_ input: [Double], period: Int32) throws -> [Double] {
        var out = [Double](repeating: 0, count: input.count)
        let rc = input.withUnsafeBufferPointer { inp in
            out.withUnsafeMutableBufferPointer { dst in
                alpha_ta_dema(inp.baseAddress, Int32(input.count), period, dst.baseAddress)
            }
        }
        guard rc == 0 else { throw FinkitError.calculationFailed }
        return out
    }

    public static func tema(_ input: [Double], period: Int32) throws -> [Double] {
        var out = [Double](repeating: 0, count: input.count)
        let rc = input.withUnsafeBufferPointer { inp in
            out.withUnsafeMutableBufferPointer { dst in
                alpha_ta_tema(inp.baseAddress, Int32(input.count), period, dst.baseAddress)
            }
        }
        guard rc == 0 else { throw FinkitError.calculationFailed }
        return out
    }

    public static func rsi(_ input: [Double], period: Int32) throws -> [Double] {
        var out = [Double](repeating: 0, count: input.count)
        let rc = input.withUnsafeBufferPointer { inp in
            out.withUnsafeMutableBufferPointer { dst in
                alpha_ta_rsi(inp.baseAddress, Int32(input.count), period, dst.baseAddress)
            }
        }
        guard rc == 0 else { throw FinkitError.calculationFailed }
        return out
    }

    public static func roc(_ input: [Double], period: Int32) throws -> [Double] {
        var out = [Double](repeating: 0, count: input.count)
        let rc = input.withUnsafeBufferPointer { inp in
            out.withUnsafeMutableBufferPointer { dst in
                alpha_ta_roc(inp.baseAddress, Int32(input.count), period, dst.baseAddress)
            }
        }
        guard rc == 0 else { throw FinkitError.calculationFailed }
        return out
    }

    public static func mom(_ input: [Double], period: Int32) throws -> [Double] {
        var out = [Double](repeating: 0, count: input.count)
        let rc = input.withUnsafeBufferPointer { inp in
            out.withUnsafeMutableBufferPointer { dst in
                alpha_ta_mom(inp.baseAddress, Int32(input.count), period, dst.baseAddress)
            }
        }
        guard rc == 0 else { throw FinkitError.calculationFailed }
        return out
    }

    public static func cmo(_ input: [Double], period: Int32) throws -> [Double] {
        var out = [Double](repeating: 0, count: input.count)
        let rc = input.withUnsafeBufferPointer { inp in
            out.withUnsafeMutableBufferPointer { dst in
                alpha_ta_cmo(inp.baseAddress, Int32(input.count), period, dst.baseAddress)
            }
        }
        guard rc == 0 else { throw FinkitError.calculationFailed }
        return out
    }

    public static func trix(_ input: [Double], period: Int32) throws -> [Double] {
        var out = [Double](repeating: 0, count: input.count)
        let rc = input.withUnsafeBufferPointer { inp in
            out.withUnsafeMutableBufferPointer { dst in
                alpha_ta_trix(inp.baseAddress, Int32(input.count), period, dst.baseAddress)
            }
        }
        guard rc == 0 else { throw FinkitError.calculationFailed }
        return out
    }

    public static func midpoint(_ input: [Double], period: Int32) throws -> [Double] {
        var out = [Double](repeating: 0, count: input.count)
        let rc = input.withUnsafeBufferPointer { inp in
            out.withUnsafeMutableBufferPointer { dst in
                alpha_ta_midpoint(inp.baseAddress, Int32(input.count), period, dst.baseAddress)
            }
        }
        guard rc == 0 else { throw FinkitError.calculationFailed }
        return out
    }

    public static func zscore(_ input: [Double], period: Int32) throws -> [Double] {
        var out = [Double](repeating: 0, count: input.count)
        let rc = input.withUnsafeBufferPointer { inp in
            out.withUnsafeMutableBufferPointer { dst in
                alpha_ta_zscore(inp.baseAddress, Int32(input.count), period, dst.baseAddress)
            }
        }
        guard rc == 0 else { throw FinkitError.calculationFailed }
        return out
    }

    public static func tsf(_ input: [Double], period: Int32) throws -> [Double] {
        var out = [Double](repeating: 0, count: input.count)
        let rc = input.withUnsafeBufferPointer { inp in
            out.withUnsafeMutableBufferPointer { dst in
                alpha_ta_tsf(inp.baseAddress, Int32(input.count), period, dst.baseAddress)
            }
        }
        guard rc == 0 else { throw FinkitError.calculationFailed }
        return out
    }

    public static func linearReg(_ input: [Double], period: Int32) throws -> [Double] {
        var out = [Double](repeating: 0, count: input.count)
        let rc = input.withUnsafeBufferPointer { inp in
            out.withUnsafeMutableBufferPointer { dst in
                alpha_ta_linear_reg(inp.baseAddress, Int32(input.count), period, dst.baseAddress)
            }
        }
        guard rc == 0 else { throw FinkitError.calculationFailed }
        return out
    }

    public static func percentRank(_ input: [Double], period: Int32) throws -> [Double] {
        var out = [Double](repeating: 0, count: input.count)
        let rc = input.withUnsafeBufferPointer { inp in
            out.withUnsafeMutableBufferPointer { dst in
                alpha_ta_percent_rank(inp.baseAddress, Int32(input.count), period, dst.baseAddress)
            }
        }
        guard rc == 0 else { throw FinkitError.calculationFailed }
        return out
    }

    public static func detectCandlestick(
        open: [Double],
        high: [Double],
        low: [Double],
        close: [Double]
    ) throws -> Int32 {
        guard open.count == high.count,
              open.count == low.count,
              open.count == close.count,
              !open.isEmpty else {
            throw FinkitError.invalidParameters
        }

        let rc = open.withUnsafeBufferPointer { o in
            high.withUnsafeBufferPointer { h in
                low.withUnsafeBufferPointer { l in
                    close.withUnsafeBufferPointer { c in
                        alpha_ta_detect_candlestick(
                            o.baseAddress, h.baseAddress, l.baseAddress, c.baseAddress,
                            Int32(open.count)
                        )
                    }
                }
            }
        }
        guard rc >= 0 else { throw FinkitError.calculationFailed }
        return rc
    }
}

@available(*, deprecated, renamed: "Finkit")
public typealias AlphaTA = Finkit

@available(*, deprecated, renamed: "FinkitError")
public typealias AlphaTAError = FinkitError
