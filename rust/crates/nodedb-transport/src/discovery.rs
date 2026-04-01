use std::sync::Arc;

use chrono::Utc;
use dashmap::DashMap;
use tokio::sync::watch;
use tracing::{info, warn};

use crate::types::{DiscoveredPeer, DiscoverySource};

const SERVICE_TYPE: &str = "_nodedb._tcp.local.";

/// Manages peer discovery via mDNS and seed peers.
pub struct DiscoveryManager {
    discovered: Arc<DashMap<String, DiscoveredPeer>>,
    my_peer_id: String,
    my_port: u16,
}

impl DiscoveryManager {
    pub fn new(peer_id: &str, port: u16) -> Self {
        DiscoveryManager {
            discovered: Arc::new(DashMap::new()),
            my_peer_id: peer_id.to_string(),
            my_port: port,
        }
    }

    /// Add seed peers (parsed from config).
    pub fn add_seed_peers(&self, seeds: &[String]) {
        for endpoint in seeds {
            let peer = DiscoveredPeer {
                peer_id: format!("seed:{}", endpoint),
                endpoint: endpoint.clone(),
                source: DiscoverySource::Seed,
                discovered_at: Utc::now(),
            };
            self.discovered.insert(peer.peer_id.clone(), peer);
        }
    }

    /// Add a peer discovered via gossip.
    pub fn add_gossip_peer(&self, peer_id: &str, endpoint: &str) {
        if peer_id == self.my_peer_id {
            return; // don't add self
        }
        let peer = DiscoveredPeer {
            peer_id: peer_id.to_string(),
            endpoint: endpoint.to_string(),
            source: DiscoverySource::Gossip,
            discovered_at: Utc::now(),
        };
        self.discovered.insert(peer_id.to_string(), peer);
    }

    /// Get all discovered peers.
    pub fn discovered_peers(&self) -> Vec<DiscoveredPeer> {
        self.discovered.iter().map(|e| e.value().clone()).collect()
    }

    /// Get a specific discovered peer.
    pub fn get_peer(&self, peer_id: &str) -> Option<DiscoveredPeer> {
        self.discovered.get(peer_id).map(|e| e.value().clone())
    }

    /// Number of discovered peers.
    pub fn peer_count(&self) -> usize {
        self.discovered.len()
    }

    /// Start mDNS discovery (publish + browse) as a background task.
    /// Returns immediately. Runs until shutdown is signalled.
    pub fn start_mdns(&self, mut shutdown: watch::Receiver<bool>) {
        let discovered = self.discovered.clone();
        let my_peer_id = self.my_peer_id.clone();
        let my_port = self.my_port;

        tokio::spawn(async move {
            if let Err(e) = run_mdns(discovered, &my_peer_id, my_port, &mut shutdown).await {
                warn!("mDNS discovery error: {}", e);
            }
        });
    }
}

/// Run mDNS publish + browse loop.
async fn run_mdns(
    discovered: Arc<DashMap<String, DiscoveredPeer>>,
    my_peer_id: &str,
    my_port: u16,
    shutdown: &mut watch::Receiver<bool>,
) -> Result<(), crate::error::TransportError> {
    let mdns = mdns_sd::ServiceDaemon::new()
        .map_err(|e| crate::error::TransportError::Discovery(e.to_string()))?;

    // Register our service
    let service_info = mdns_sd::ServiceInfo::new(
        SERVICE_TYPE,
        &format!("nodedb-{}", &my_peer_id[..8.min(my_peer_id.len())]),
        &format!("{}.local.", &my_peer_id[..8.min(my_peer_id.len())]),
        "",
        my_port,
        Some(std::collections::HashMap::from([
            ("peer_id".to_string(), my_peer_id.to_string()),
            ("version".to_string(), "1".to_string()),
        ])),
    )
    .map_err(|e| crate::error::TransportError::Discovery(e.to_string()))?;

    mdns.register(service_info)
        .map_err(|e| crate::error::TransportError::Discovery(e.to_string()))?;

    info!("mDNS: published {} on port {}", my_peer_id, my_port);

    // Browse for other peers
    let receiver = mdns
        .browse(SERVICE_TYPE)
        .map_err(|e| crate::error::TransportError::Discovery(e.to_string()))?;

    let my_peer_id = my_peer_id.to_string();
    loop {
        tokio::select! {
            event = tokio::task::spawn_blocking({
                let receiver = receiver.clone();
                move || receiver.recv_timeout(std::time::Duration::from_secs(2))
            }) => {
                match event {
                    Ok(Ok(mdns_sd::ServiceEvent::ServiceResolved(info))) => {
                        let peer_id = info.get_properties()
                            .get("peer_id")
                            .map(|v| v.val_str().to_string())
                            .unwrap_or_default();

                        if peer_id.is_empty() || peer_id == my_peer_id {
                            continue;
                        }

                        let port = info.get_port();
                        let addrs: Vec<_> = info.get_addresses().iter().collect();
                        if let Some(addr) = addrs.first() {
                            let endpoint = format!("wss://{}:{}", addr, port);
                            info!("mDNS: discovered peer {} at {}", peer_id, endpoint);
                            let peer = DiscoveredPeer {
                                peer_id: peer_id.clone(),
                                endpoint,
                                source: DiscoverySource::Mdns,
                                discovered_at: Utc::now(),
                            };
                            discovered.insert(peer_id, peer);
                        }
                    }
                    Ok(Ok(_)) => {} // other events (searching, etc.)
                    Ok(Err(_)) => {} // timeout, keep looping
                    Err(_) => break, // join error
                }
            }
            _ = shutdown.changed() => {
                info!("mDNS: shutting down");
                let _ = mdns.shutdown();
                break;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_peer_resolution() {
        let mgr = DiscoveryManager::new("test-peer", 9400);
        mgr.add_seed_peers(&[
            "wss://192.168.1.10:9400".to_string(),
            "wss://192.168.1.11:9400".to_string(),
        ]);
        assert_eq!(mgr.peer_count(), 2);
        let peers = mgr.discovered_peers();
        assert!(peers.iter().all(|p| p.source == DiscoverySource::Seed));
    }

    #[test]
    fn gossip_peer_added() {
        let mgr = DiscoveryManager::new("my-peer", 9400);
        mgr.add_gossip_peer("other-peer", "wss://10.0.0.5:9400");
        assert_eq!(mgr.peer_count(), 1);
        let peer = mgr.get_peer("other-peer").unwrap();
        assert_eq!(peer.source, DiscoverySource::Gossip);
        assert_eq!(peer.endpoint, "wss://10.0.0.5:9400");
    }

    #[test]
    fn self_not_added_via_gossip() {
        let mgr = DiscoveryManager::new("my-peer", 9400);
        mgr.add_gossip_peer("my-peer", "wss://127.0.0.1:9400");
        assert_eq!(mgr.peer_count(), 0);
    }

    #[test]
    fn gossip_overwrites_existing() {
        let mgr = DiscoveryManager::new("my-peer", 9400);
        mgr.add_gossip_peer("other", "wss://10.0.0.1:9400");
        mgr.add_gossip_peer("other", "wss://10.0.0.2:9400");
        assert_eq!(mgr.peer_count(), 1);
        let peer = mgr.get_peer("other").unwrap();
        assert_eq!(peer.endpoint, "wss://10.0.0.2:9400");
    }
}
