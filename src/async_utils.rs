use std::sync::LazyLock;

use tokio::runtime::Runtime;

pub fn tokio_rt() -> &'static Runtime {
    static TOKIO_RT: LazyLock<Runtime> =
        LazyLock::new(|| Runtime::new().expect("Failed to create Tokio runtime"));
    &TOKIO_RT
}

/// Spawn a future that will run on a Tokio worker thread.
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
    // Note that if we ever need to spawn a future that runs in GTK's main thread,
    // we can use something like below directly.
    gtk::glib::spawn_future_local(async {
        if let Ok(result) = rx.await {
            callback(result);
        }
    });
}
