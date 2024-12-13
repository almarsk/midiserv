use std::sync::Arc;

use flume::Receiver;
use reqwest::Client;
use tokio::{runtime::Runtime, sync::Mutex};

use crate::Login;

pub fn logout_task(
    rt: &Runtime,
    shutdown: Receiver<bool>,
    logout: Receiver<()>,
    login_data: Arc<Mutex<Option<Login>>>,
) {
    rt.spawn(async move {
        loop {
            tokio::select! {
                shutdown_option = shutdown.recv_async() => {
                    if let Ok(shutdown) = shutdown_option {
                        if shutdown {
                            break;
                        }
                    }
                }
                _ = logout.recv_async() => {
                    if let Some(ref login) = *login_data.lock().await {
                        let _ = Client::new().get(format!("http://{}/logout", &login.url)).send().await;
                    }
                }
            }
        }
    });
}
