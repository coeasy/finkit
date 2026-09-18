package com.finkit;

/**
 * Dependency-free host smoke runner for the shared engine contract.
 *
 * <p>Run it with the compiled JNI library in {@code finkit.native.path}; the
 * request and expected values intentionally match
 * {@code tests/contracts/engine_contract_v1.json}.</p>
 */
public final class ContractConformance {
    private ContractConformance() {
    }

    private static void requireContains(String payload, String fragment) {
        if (!payload.contains(fragment)) {
            throw new AssertionError("missing fragment " + fragment + " in " + payload);
        }
    }

    public static void main(String[] args) {
        String operation = Indicators.operationExecuteJson(
                "{\"operation\":\"SMA\",\"input_order\":[\"CLOSE\"],"
                        + "\"inputs\":{\"CLOSE\":[1.0,2.0,3.0,4.0]},\"params\":[2.0]}");
        requireContains(operation, "\"SMA\":[1.0,1.5,2.25,3.125]");

        String formula = Indicators.formulaEvalContractJson(
                "CLOSE + 1",
                "tdx",
                new double[] {1.0, 2.0, 3.0, 4.0},
                new double[] {1.0, 2.0, 3.0, 4.0},
                new double[] {1.0, 2.0, 3.0, 4.0},
                new double[] {1.0, 2.0, 3.0, 4.0},
                new double[] {10.0, 10.0, 10.0, 10.0});
        requireContains(formula, "\"__PRIMARY__\":[2.0,3.0,4.0,5.0]");

        String factor = Indicators.factorExecuteJson(
                "{\"schema_version\":1,\"targets\":[\"momentum_5\"],"
                        + "\"scope\":\"FIXTURE@1d\",\"data_revision\":1,"
                        + "\"inputs\":{\"close\":[1.0,2.0,3.0,4.0,5.0,6.0]}}");
        requireContains(factor, "\"momentum_5\":[null,null,null,null,null,5.0]");

        String composite = Indicators.compositeExecuteJson(
                "{\"schema_version\":1,\"scope\":\"FIXTURE@1d\","
                        + "\"data_revision\":1,\"inputs\":{\"close\":[1.0,2.0,3.0,4.0,5.0]},"
                        + "\"definitions\":[{\"name\":\"sma3\",\"function\":\"sma\","
                        + "\"inputs\":[\"close\"],\"params\":[3]}],\"outputs\":[\"sma3\"]}");
        requireContains(composite, "\"sma3\":[null,null,2.0,3.0,4.0]");
    }
}
