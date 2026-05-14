#![allow(dead_code)]

use tokio::sync::{broadcast, mpsc, oneshot};

use super::{enums::SystemResourcesQuery, structs::*};

pub fn subscribe_events() -> Option<broadcast::Receiver<super::enums::SystemResourcesEvent>> {
    super::resources::SYSTEM_RESOURCES_EVENT_SENDER
        .get()
        .map(|tx| tx.subscribe())
}

fn get_system_resources_query_sender() -> Option<&'static mpsc::Sender<SystemResourcesQuery>> {
    super::resources::SYSTEM_RESOURCES_QUERY_SENDER.get()
}

/// Get the current system snapshot.
pub async fn get_snapshot() -> Option<SystemSnapshot> {
    let (tx, rx) = oneshot::channel();
    let _ = get_system_resources_query_sender()?
        .send(SystemResourcesQuery::Snapshot { response: tx })
        .await;
    rx.await.unwrap_or(None)
}

pub async fn get_cpu() -> Option<CpuSnapshot> {
    let (tx, rx) = oneshot::channel();
    let _ = get_system_resources_query_sender()?
        .send(SystemResourcesQuery::Cpu { response: tx })
        .await;
    rx.await.unwrap_or(None)
}

pub async fn get_memory() -> Option<MemorySnapshot> {
    let (tx, rx) = oneshot::channel();
    let _ = get_system_resources_query_sender()?
        .send(SystemResourcesQuery::Memory { response: tx })
        .await;
    rx.await.unwrap_or(None)
}

pub async fn get_disks() -> Vec<DiskSnapshot> {
    let Some(tx_ref) = get_system_resources_query_sender() else {
        return Vec::new();
    };
    let (tx, rx) = oneshot::channel();
    let _ = tx_ref
        .send(SystemResourcesQuery::Disks { response: tx })
        .await;
    rx.await.unwrap_or_default()
}

pub async fn get_networks() -> Vec<NetworkSnapshot> {
    let Some(tx_ref) = get_system_resources_query_sender() else {
        return Vec::new();
    };
    let (tx, rx) = oneshot::channel();
    let _ = tx_ref
        .send(SystemResourcesQuery::Networks { response: tx })
        .await;
    rx.await.unwrap_or_default()
}

pub async fn get_gpus() -> Vec<GpuSnapshot> {
    let Some(tx_ref) = get_system_resources_query_sender() else {
        return Vec::new();
    };
    let (tx, rx) = oneshot::channel();
    let _ = tx_ref
        .send(SystemResourcesQuery::Gpus { response: tx })
        .await;
    rx.await.unwrap_or_default()
}

pub async fn get_battery() -> Option<BatterySnapshot> {
    let (tx, rx) = oneshot::channel();
    let _ = get_system_resources_query_sender()?
        .send(SystemResourcesQuery::Battery { response: tx })
        .await;
    rx.await.unwrap_or(None)
}

pub async fn get_host_info() -> Option<HostInfo> {
    let (tx, rx) = oneshot::channel();
    let _ = get_system_resources_query_sender()?
        .send(SystemResourcesQuery::HostInfo { response: tx })
        .await;
    rx.await.unwrap_or(None)
}

pub async fn get_temperatures() -> Vec<ThermalSnapshot> {
    let Some(tx_ref) = get_system_resources_query_sender() else {
        return Vec::new();
    };
    let (tx, rx) = oneshot::channel();
    let _ = tx_ref
        .send(SystemResourcesQuery::Temperatures { response: tx })
        .await;
    rx.await.unwrap_or_default()
}
