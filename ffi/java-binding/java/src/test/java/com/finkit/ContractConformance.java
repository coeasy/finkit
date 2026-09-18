package com.finkit;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

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

    private static int countOccurrences(String payload, String fragment) {
        int count = 0;
        int offset = 0;
        while ((offset = payload.indexOf(fragment, offset)) >= 0) {
            count++;
            offset += fragment.length();
        }
        return count;
    }

    private static Path findFixture(String name) {
        Path[] candidates = new Path[] {
                Paths.get("tests", "contracts", name),
                Paths.get("..", "..", "tests", "contracts", name),
                Paths.get("..", "..", "..", "tests", "contracts", name)
        };
        for (Path candidate : candidates) {
            if (Files.isRegularFile(candidate)) {
                return candidate;
            }
        }
        throw new AssertionError("fixture not found: " + name);
    }

    @SuppressWarnings("unchecked")
    private static void runTalibNumericContract() {
        final Map<String, Object> fixture;
        try {
            String text = new String(Files.readAllBytes(findFixture("talib_numeric_contract_v1.json")),
                    StandardCharsets.UTF_8);
            fixture = (Map<String, Object>) MiniJson.parse(text);
        } catch (IOException error) {
            throw new AssertionError("read TA-Lib numeric contract", error);
        }
        requireEquals("talib_0_8_0", fixture.get("semantic_profile"), "numeric profile");
        List<Object> vectors = (List<Object>) fixture.get("vectors");
        requireEquals(201, vectors.size(), "numeric vector count");

        for (Object rawVector : vectors) {
            Map<String, Object> vector = (Map<String, Object>) rawVector;
            Map<String, Object> request = new LinkedHashMap<>();
            request.put("operation", vector.get("operation"));
            request.put("semantic_profile", fixture.get("semantic_profile"));
            request.put("input_order", vector.get("input_order"));
            request.put("inputs", fixture.get("inputs"));
            request.put("params", vector.get("params"));
            String operation = (String) vector.get("operation");
            Map<String, Object> payload = (Map<String, Object>) MiniJson.parse(
                    Indicators.operationExecuteJson(MiniJson.stringify(request)));
            if (payload.containsKey("error") && payload.get("error") != null) {
                throw new AssertionError(operation + ": " + MiniJson.stringify(payload));
            }
            Map<String, Object> values = (Map<String, Object>) payload.get("values");
            Map<String, Object> expectedOutputs = (Map<String, Object>) vector.get("expected");
            Map<String, Object> tolerance = (Map<String, Object>) vector.get("tolerance");
            double atol = ((Number) tolerance.get("atol")).doubleValue();
            double rtol = ((Number) tolerance.get("rtol")).doubleValue();
            for (Map.Entry<String, Object> output : expectedOutputs.entrySet()) {
                List<Object> expected = (List<Object>) output.getValue();
                List<Object> actual = (List<Object>) values.get(output.getKey());
                if (actual == null || actual.size() != expected.size()) {
                    throw new AssertionError(operation + "/" + output.getKey() + ": length or output mismatch");
                }
                for (int index = 0; index < expected.size(); index++) {
                    Object expectedValue = expected.get(index);
                    Object actualValue = actual.get(index);
                    if (expectedValue == null) {
                        if (actualValue != null) {
                            throw new AssertionError(operation + "/" + output.getKey() + "[" + index + "]: expected null");
                        }
                        continue;
                    }
                    if (!(actualValue instanceof Number)) {
                        throw new AssertionError(operation + "/" + output.getKey() + "[" + index + "]: unexpected null");
                    }
                    double want = ((Number) expectedValue).doubleValue();
                    double got = ((Number) actualValue).doubleValue();
                    double limit = atol + rtol * Math.abs(want);
                    if (Math.abs(got - want) > limit) {
                        throw new AssertionError(operation + "/" + output.getKey() + "[" + index
                                + "]: error " + Math.abs(got - want) + " > " + limit);
                    }
                }
            }
        }
    }

    private static void requireEquals(Object expected, Object actual, String label) {
        if (!expected.equals(actual)) {
            throw new AssertionError(label + ": got " + actual + " want " + expected);
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

        String catalog = Indicators.operationCatalogJson();
        requireContains(catalog, "\"schema_version\":1");
        requireContains(catalog, "\"name\":\"SMA\"");
        requireContains(catalog, "talib_0_8_0");
        int currentProfileCount = countOccurrences(
                catalog, "\"profile_output_contracts\":{\"talib_0_8_0\"");
        if (catalog.contains("talib_0_7_1") || currentProfileCount != 201) {
            throw new AssertionError("TA-Lib catalog must expose exactly 201 current-profile operations");
        }

        runTalibNumericContract();
    }

    /** Minimal JSON codec for the dependency-free contract runner. */
    private static final class MiniJson {
        private final String text;
        private int index;

        private MiniJson(String text) {
            this.text = text;
        }

        static Object parse(String text) {
            MiniJson parser = new MiniJson(text);
            Object value = parser.value();
            parser.whitespace();
            if (parser.index != text.length()) {
                throw new IllegalArgumentException("trailing JSON at " + parser.index);
            }
            return value;
        }

        static String stringify(Object value) {
            StringBuilder builder = new StringBuilder();
            write(value, builder);
            return builder.toString();
        }

        private static void write(Object value, StringBuilder builder) {
            if (value == null) {
                builder.append("null");
            } else if (value instanceof Map) {
                Map<?, ?> map = (Map<?, ?>) value;
                builder.append('{');
                boolean first = true;
                for (Map.Entry<?, ?> entry : map.entrySet()) {
                    if (!first) {
                        builder.append(',');
                    }
                    first = false;
                    writeString(String.valueOf(entry.getKey()), builder);
                    builder.append(':');
                    write(entry.getValue(), builder);
                }
                builder.append('}');
            } else if (value instanceof List) {
                List<?> list = (List<?>) value;
                builder.append('[');
                for (int i = 0; i < list.size(); i++) {
                    if (i != 0) {
                        builder.append(',');
                    }
                    write(list.get(i), builder);
                }
                builder.append(']');
            } else if (value instanceof String) {
                writeString((String) value, builder);
            } else if (value instanceof Number || value instanceof Boolean) {
                builder.append(value);
            } else {
                throw new IllegalArgumentException("unsupported JSON value: " + value.getClass());
            }
        }

        private static void writeString(String value, StringBuilder builder) {
            builder.append('"');
            for (int i = 0; i < value.length(); i++) {
                char character = value.charAt(i);
                switch (character) {
                    case '"': builder.append("\\\""); break;
                    case '\\': builder.append("\\\\"); break;
                    case '\n': builder.append("\\n"); break;
                    case '\r': builder.append("\\r"); break;
                    case '\t': builder.append("\\t"); break;
                    default: builder.append(character); break;
                }
            }
            builder.append('"');
        }

        private Object value() {
            whitespace();
            if (index >= text.length()) {
                throw new IllegalArgumentException("unexpected end of JSON");
            }
            switch (text.charAt(index)) {
                case '{': return object();
                case '[': return array();
                case '"': return string();
                case 't': return literal("true", Boolean.TRUE);
                case 'f': return literal("false", Boolean.FALSE);
                case 'n': return literal("null", null);
                default: return number();
            }
        }

        private Map<String, Object> object() {
            Map<String, Object> result = new LinkedHashMap<>();
            index++;
            whitespace();
            if (consume('}')) {
                return result;
            }
            while (true) {
                whitespace();
                String key = string();
                whitespace();
                require(':');
                result.put(key, value());
                whitespace();
                if (consume('}')) {
                    return result;
                }
                require(',');
            }
        }

        private List<Object> array() {
            List<Object> result = new ArrayList<>();
            index++;
            whitespace();
            if (consume(']')) {
                return result;
            }
            while (true) {
                result.add(value());
                whitespace();
                if (consume(']')) {
                    return result;
                }
                require(',');
            }
        }

        private String string() {
            require('"');
            StringBuilder result = new StringBuilder();
            while (index < text.length()) {
                char character = text.charAt(index++);
                if (character == '"') {
                    return result.toString();
                }
                if (character != '\\') {
                    result.append(character);
                    continue;
                }
                if (index >= text.length()) {
                    throw new IllegalArgumentException("unterminated escape");
                }
                char escaped = text.charAt(index++);
                char decoded;
                switch (escaped) {
                    case '"': decoded = '"'; break;
                    case '\\': decoded = '\\'; break;
                    case '/': decoded = '/'; break;
                    case 'b': decoded = '\b'; break;
                    case 'f': decoded = '\f'; break;
                    case 'n': decoded = '\n'; break;
                    case 'r': decoded = '\r'; break;
                    case 't': decoded = '\t'; break;
                    case 'u':
                        decoded = (char) Integer.parseInt(text.substring(index, index + 4), 16);
                        break;
                    default: throw new IllegalArgumentException("invalid escape: " + escaped);
                }
                result.append(decoded);
                if (escaped == 'u') {
                    index += 4;
                }
            }
            throw new IllegalArgumentException("unterminated string");
        }

        private Object number() {
            int start = index;
            while (index < text.length() && "-+0123456789.eE".indexOf(text.charAt(index)) >= 0) {
                index++;
            }
            return Double.valueOf(text.substring(start, index));
        }

        private Object literal(String literal, Object value) {
            if (!text.startsWith(literal, index)) {
                throw new IllegalArgumentException("invalid literal at " + index);
            }
            index += literal.length();
            return value;
        }

        private void whitespace() {
            while (index < text.length() && Character.isWhitespace(text.charAt(index))) {
                index++;
            }
        }

        private boolean consume(char character) {
            if (index < text.length() && text.charAt(index) == character) {
                index++;
                return true;
            }
            return false;
        }

        private void require(char character) {
            whitespace();
            if (!consume(character)) {
                throw new IllegalArgumentException("expected '" + character + "' at " + index);
            }
        }
    }
}
