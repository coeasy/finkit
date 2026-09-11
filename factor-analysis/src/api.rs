use crate::analysis::WeightConfig;
use crate::data::{AssetId, GroupId, PanelIndex, ResearchFrame};
use crate::error::{ResearchError, ResearchResult};
use crate::prepare::QuantizeConfig;
use crate::report::{AnalysisMode, FactorStudy, FactorStudyReport};
use serde::{Deserialize, Serialize};

/// Version of the language-neutral factor research request/response contract.
pub const FACTOR_STUDY_SCHEMA_VERSION: u32 = 1;

fn default_schema_version() -> u32 {
    FACTOR_STUDY_SCHEMA_VERSION
}

fn default_quantiles() -> u16 {
    5
}

fn default_true() -> bool {
    true
}

fn default_mode() -> AnalysisMode {
    AnalysisMode::Native
}

/// Language-neutral request for a complete panel-aware factor study.
///
/// Every binding (Python, Node, Java, C/C++, Go, .NET, Swift and WASM) can
/// serialize this structure and therefore share the exact same validation and
/// computation path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FactorStudyRequest {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub timestamps: Vec<i64>,
    pub assets: Vec<u32>,
    pub factor: Vec<f64>,
    pub prices: Vec<f64>,
    pub periods: Vec<usize>,
    #[serde(default = "default_quantiles")]
    pub quantiles: u16,
    #[serde(default)]
    pub groups: Option<Vec<u32>>,
    #[serde(default)]
    pub zero_aware: bool,
    #[serde(default)]
    pub quantize_by_group: bool,
    #[serde(default)]
    pub group_neutral: bool,
    #[serde(default = "default_true")]
    pub demeaned: bool,
    #[serde(default)]
    pub equal_weight: bool,
    #[serde(default = "default_mode")]
    pub mode: AnalysisMode,
}

/// Stable error payload used at every FFI boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchApiError {
    pub code: String,
    pub message: String,
}

impl ResearchApiError {
    fn invalid_request(message: impl Into<String>) -> Self {
        Self {
            code: "invalid_request".to_string(),
            message: message.into(),
        }
    }

    fn computation(error: ResearchError) -> Self {
        Self {
            code: "computation_failed".to_string(),
            message: error.to_string(),
        }
    }
}

/// Stable language-neutral response envelope.
///
/// Returning an envelope rather than throwing binding-specific exceptions
/// keeps C ABI, JNI, N-API, P/Invoke, cgo and WASM behavior identical.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FactorStudyResponse {
    pub schema_version: u32,
    pub library_version: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report: Option<FactorStudyReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ResearchApiError>,
}

impl FactorStudyResponse {
    fn success(report: FactorStudyReport) -> Self {
        Self {
            schema_version: FACTOR_STUDY_SCHEMA_VERSION,
            library_version: env!("CARGO_PKG_VERSION").to_string(),
            ok: true,
            report: Some(report),
            error: None,
        }
    }

    fn failure(error: ResearchApiError) -> Self {
        Self {
            schema_version: FACTOR_STUDY_SCHEMA_VERSION,
            library_version: env!("CARGO_PKG_VERSION").to_string(),
            ok: false,
            report: None,
            error: Some(error),
        }
    }
}

fn validate_request(request: &FactorStudyRequest) -> Result<(), ResearchApiError> {
    if request.schema_version != FACTOR_STUDY_SCHEMA_VERSION {
        return Err(ResearchApiError {
            code: "unsupported_schema".to_string(),
            message: format!(
                "unsupported factor study schema version {}; expected {}",
                request.schema_version, FACTOR_STUDY_SCHEMA_VERSION
            ),
        });
    }
    let rows = request.timestamps.len();
    if rows == 0 {
        return Err(ResearchApiError::invalid_request(
            "factor study input must contain at least one row",
        ));
    }
    for (name, actual) in [
        ("assets", request.assets.len()),
        ("factor", request.factor.len()),
        ("prices", request.prices.len()),
    ] {
        if actual != rows {
            return Err(ResearchApiError::invalid_request(format!(
                "{name} length mismatch: expected {rows}, got {actual}"
            )));
        }
    }
    if let Some(groups) = &request.groups {
        if groups.len() != rows {
            return Err(ResearchApiError::invalid_request(format!(
                "groups length mismatch: expected {rows}, got {}",
                groups.len()
            )));
        }
    }
    if request.periods.is_empty() || request.periods.iter().any(|&period| period == 0) {
        return Err(ResearchApiError::invalid_request(
            "periods must contain at least one positive horizon",
        ));
    }
    if request.quantiles == 0 {
        return Err(ResearchApiError::invalid_request(
            "quantiles must be greater than zero",
        ));
    }
    if (request.quantize_by_group || request.group_neutral) && request.groups.is_none() {
        return Err(ResearchApiError::invalid_request(
            "groups are required when quantize_by_group or group_neutral is enabled",
        ));
    }
    Ok(())
}

/// Execute a factor study from the canonical request contract.
pub fn run_factor_study(request: &FactorStudyRequest) -> ResearchResult<FactorStudyReport> {
    let assets = request.assets.iter().copied().map(AssetId).collect();
    let index = PanelIndex::new(request.timestamps.clone(), assets)?;
    let mut frame = ResearchFrame::new(index);
    frame.add_numeric("factor", "factor", request.factor.clone())?;
    frame.add_numeric("price", "market", request.prices.clone())?;
    if let Some(groups) = &request.groups {
        frame.add_group("group", groups.iter().copied().map(GroupId).collect())?;
    }

    let group_name = || Some("group".to_string());
    FactorStudy::new(&frame, "factor", "price", request.periods.clone())
        .mode(request.mode)
        .quantize_config(QuantizeConfig {
            quantiles: request.quantiles,
            by_group: request.quantize_by_group.then(group_name),
            zero_aware: request.zero_aware,
        })
        .weight_config(WeightConfig {
            demeaned: request.demeaned,
            group_adjust: request.group_neutral.then(group_name),
            equal_weight: request.equal_weight,
        })
        .full_report()
}

/// Execute a canonical request and convert all validation/computation failures
/// into the stable response envelope.
pub fn run_factor_study_response(request: &FactorStudyRequest) -> FactorStudyResponse {
    if let Err(error) = validate_request(request) {
        return FactorStudyResponse::failure(error);
    }
    match run_factor_study(request) {
        Ok(report) => FactorStudyResponse::success(report),
        Err(error) => FactorStudyResponse::failure(ResearchApiError::computation(error)),
    }
}

/// JSON-in/JSON-out entry point shared by every language binding.
pub fn run_factor_study_json(request_json: &str) -> String {
    let response = match serde_json::from_str::<FactorStudyRequest>(request_json) {
        Ok(request) => run_factor_study_response(&request),
        Err(error) => FactorStudyResponse::failure(ResearchApiError {
            code: "invalid_json".to_string(),
            message: error.to_string(),
        }),
    };
    serde_json::to_string(&response).unwrap_or_else(|error| {
        format!(
            "{{\"schema_version\":{},\"library_version\":\"{}\",\"ok\":false,\"error\":{{\"code\":\"serialization_failed\",\"message\":{:?}}}}}",
            FACTOR_STUDY_SCHEMA_VERSION,
            env!("CARGO_PKG_VERSION"),
            error.to_string()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> FactorStudyRequest {
        FactorStudyRequest {
            schema_version: FACTOR_STUDY_SCHEMA_VERSION,
            timestamps: vec![1, 1, 1, 2, 2, 2, 3, 3, 3],
            assets: vec![1, 2, 3, 1, 2, 3, 1, 2, 3],
            factor: vec![1.0, 2.0, 3.0, 1.0, 2.0, 3.0, 1.0, 2.0, 3.0],
            prices: vec![10.0, 10.0, 10.0, 11.0, 12.0, 13.0, 12.0, 14.0, 16.0],
            periods: vec![1],
            quantiles: 3,
            groups: None,
            zero_aware: false,
            quantize_by_group: false,
            group_neutral: false,
            demeaned: true,
            equal_weight: false,
            mode: AnalysisMode::Native,
        }
    }

    #[test]
    fn canonical_request_runs_complete_study() {
        let response = run_factor_study_response(&request());
        assert!(response.ok);
        let report = response.report.unwrap();
        assert_eq!(report.periods, vec![1]);
        assert_eq!(report.quantiles, 3);
    }

    #[test]
    fn json_boundary_never_throws_binding_specific_errors() {
        let response: FactorStudyResponse =
            serde_json::from_str(&run_factor_study_json("not json")).unwrap();
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, "invalid_json");
    }

    #[test]
    fn schema_and_lengths_are_validated_before_compute() {
        let mut invalid = request();
        invalid.assets.pop();
        let response = run_factor_study_response(&invalid);
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, "invalid_request");

        let mut unsupported = request();
        unsupported.schema_version = FACTOR_STUDY_SCHEMA_VERSION + 1;
        let response = run_factor_study_response(&unsupported);
        assert_eq!(response.error.unwrap().code, "unsupported_schema");
    }
}
