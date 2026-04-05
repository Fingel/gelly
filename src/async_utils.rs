use std::sync::OnceLock;

use tokio::runtime::Runtime;

pub fn tokio_rt() -> &'static Runtime {
    static TOKIO_RT: OnceLock<Runtime> = OnceLock::new();
    TOKIO_RT.get_or_init(|| Runtime::new().expect("Failed to create Tokio runtime"))
}

/// Run future on a tokio worker thread and call callback with the result on
/// the glib main thread. For sync call sites, for example most
/// GTK interactions on the main thread.
pub fn spawn_tokio<F, T>(future: F, callback: impl FnOnce(T) + 'static)
where
    F: std::future::Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let (tx, rx) = tokio::sync::oneshot::channel();
    tokio_rt().spawn(async {
        let result = future.await;
        let _ = tx.send(result);
    });
    gtk::glib::spawn_future_local(async {
        if let Ok(result) = rx.await {
            callback(result);
        }
    });
}

/// Run future on a tokio worker thread and return its result directly.
/// for async call sites, for example as part of a multi await chain.
pub async fn run_on_tokio<F, T>(future: F) -> T
where
    F: std::future::Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let (tx, rx) = tokio::sync::oneshot::channel();
    tokio_rt().spawn(async move {
        let _ = tx.send(future.await);
    });
    rx.await.expect("tokio task channel closed")
}
