import Foundation
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
