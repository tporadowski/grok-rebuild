//! Terminal protocol queries and the agent connection share one wait.
//!
//! The first frame needs both. Starting the connection only after the queries return makes that
//! frame wait for the sum. The queries run on their own thread while this task polls the connection.
//!
//! That thread is not a Tokio worker. `tokio::signal` inside terminal init panics there unless the
//! caller enters a `tokio::runtime::Handle` captured on this task.

/// Run `queries` beside `connect`. The join waits for both. Wall time is the longer of the two.
pub async fn overlap_terminal_queries_with_connect<Q, C, T, U>(queries: Q, connect: C) -> (T, U)
where
    Q: FnOnce() -> T + Send + 'static,
    C: std::future::Future<Output = U>,
    T: Send + 'static,
{
    let query_thread = std::thread::Builder::new()
        .name("terminal-queries".into())
        .spawn(queries)
        .expect("terminal query thread");
    let connected = connect.await;
    let queried = query_thread
        .join()
        .unwrap_or_else(|err| std::panic::resume_unwind(err));
    (queried, connected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[tokio::test]
    async fn terminal_queries_overlap_the_agent_connection() {
        let origin = Instant::now();
        let (query_window, connect_window) = overlap_terminal_queries_with_connect(
            move || {
                let start = origin.elapsed();
                std::thread::sleep(Duration::from_millis(100));
                let end = origin.elapsed();
                (start, end)
            },
            async move {
                let start = origin.elapsed();
                tokio::time::sleep(Duration::from_millis(100)).await;
                let end = origin.elapsed();
                (start, end)
            },
        )
        .await;
        let (query_start, query_end) = query_window;
        let (connect_start, connect_end) = connect_window;
        assert!(
            query_start < Duration::from_millis(80),
            "queries started late: {query_start:?}"
        );
        assert!(
            connect_start < Duration::from_millis(80),
            "connection started late: {connect_start:?}"
        );
        assert!(
            query_start < connect_end && connect_start < query_end,
            "the waits ran one after the other: queries {query_start:?}..{query_end:?}, connect {connect_start:?}..{connect_end:?}"
        );
        let finished = query_end.max(connect_end);
        assert!(
            finished < Duration::from_millis(180),
            "first frame waited for the sum ({finished:?})"
        );
    }
}
