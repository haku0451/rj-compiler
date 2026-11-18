package rj.runtime;

public final class Result<T, E> {
    private final T value;
    private final E error;
    private final boolean ok;

    private Result(T value, E error, boolean ok) {
        this.value = value;
        this.error = error;
        this.ok = ok;
    }

    public static <T, E> Result<T, E> ok(T value) {
        return new Result<>(value, null, true);
    }

    public static <T, E> Result<T, E> err(E error) {
        return new Result<>(null, error, false);
    }

    public boolean isOk() {
        return ok;
    }

    public boolean isErr() {
        return !ok;
    }

    public T value() {
        return value;
    }

    public E error() {
        return error;
    }
}
