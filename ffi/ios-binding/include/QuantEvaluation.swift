import Foundation
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
