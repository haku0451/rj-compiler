package rj.runtime;

import java.util.ArrayList;
import java.util.List;

public final class RjRuntime {

    public static String concat(String a, String b) {
        return a + b;
    }

    public static Iterable<Integer> range(int n) {
        List<Integer> values = new ArrayList<>(Math.max(n, 0));
        for (int i = 0; i < n; i++) {
            values.add(i);
        }
        return values;
    }

    public static Object unwrapResult(Result<?, ?> result) {
        if (result == null) {
            throw new RuntimeException("Result is null");
        }
        if (result.isErr()) {
            throw new RuntimeException(String.valueOf(result.error()));
        }
        return result.value();
    }

    public static String castString(Object value) {
        if (value instanceof String) {
            return (String) value;
        }
        throw castError("str", value);
    }

    public static int unboxInt(Object value) {
        if (value instanceof Integer) {
            return ((Integer) value).intValue();
        }
        throw castError("int", value);
    }

    private static ClassCastException castError(String expected, Object value) {
        String actual = value == null ? "null" : value.getClass().getName();
        return new ClassCastException("expected " + expected + ", got " + actual);
    }
}
