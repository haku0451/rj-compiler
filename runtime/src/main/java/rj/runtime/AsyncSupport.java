package rj.runtime;

import java.util.concurrent.CompletableFuture;

public final class AsyncSupport {

    public static CompletableFuture<Object> complete(CompletableFuture<Object> future, Object value) {
        future.complete(value);
        return future;
    }
}
