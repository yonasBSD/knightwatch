use tokio::sync::{broadcast, mpsc, oneshot};

use super::{enums::SystemdQuery, structs::*};

#[cfg(target_os = "linux")]
pub fn subscribe_events() -> Option<broadcast::Receiver<super::enums::SystemdEvent>> {
    super::monitor::SYSTEMD_EVENT_SENDER
        .get()
        .map(|tx| tx.subscribe())
}

#[cfg(not(target_os = "linux"))]
pub fn subscribe_events() -> Option<broadcast::Receiver<super::enums::SystemdEvent>> {
    None
}

#[cfg(target_os = "linux")]
fn get_systemd_query_sender() -> Option<&'static mpsc::Sender<SystemdQuery>> {
    super::monitor::SYSTEMD_QUERY_SENDER.get()
}

#[cfg(not(target_os = "linux"))]
fn get_systemd_query_sender() -> Option<&'static mpsc::Sender<SystemdQuery>> {
    None
}

pub async fn get_snapshot() -> Option<SystemdSnapshot> {
    let (tx, rx) = oneshot::channel();
    let _ = get_systemd_query_sender()?
        .send(SystemdQuery::Snapshot { response: tx })
        .await;
    rx.await.unwrap_or(None)
}

pub async fn get_unit(unit_name: String) -> Option<UnitSnapshot> {
    let (tx, rx) = oneshot::channel();
    let _ = get_systemd_query_sender()?
        .send(SystemdQuery::Unit {
            unit_name,
            response: tx,
        })
        .await;
    rx.await.unwrap_or(None)
}

pub async fn get_units_by_active_state(state: super::enums::UnitActiveState) -> Vec<UnitSnapshot> {
    let Some(tx_ref) = get_systemd_query_sender() else {
        return Vec::new();
    };
    let (tx, rx) = oneshot::channel();
    let _ = tx_ref
        .send(SystemdQuery::ByActiveState {
            state,
            response: tx,
        })
        .await;
    rx.await.unwrap_or_default()
}

pub async fn get_failed_units() -> Vec<UnitSnapshot> {
    let Some(tx_ref) = get_systemd_query_sender() else {
        return Vec::new();
    };
    let (tx, rx) = oneshot::channel();
    let _ = tx_ref
        .send(SystemdQuery::ByActiveState {
            state: super::enums::UnitActiveState::Failed,
            response: tx,
        })
        .await;
    rx.await.unwrap_or_default()
}
