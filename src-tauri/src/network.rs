use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub id: String,           // Adapter GUID or name
    pub name: String,         // Friendly name (e.g., "ProtonVPN", "Ethernet")
    pub ipv4: Option<String>, // IPv4 address
    pub is_up: bool,          // Connection status
}

/// Get all network interfaces with IPv4 addresses.
/// Filters out loopback and link-local addresses.
/// Sorts connected interfaces first, then alphabetically by name.
#[cfg(target_os = "windows")]
pub fn get_network_interfaces() -> Result<Vec<NetworkInterface>, String> {
    let adapters = ipconfig::get_adapters()
        .map_err(|e| format!("Failed to enumerate network adapters: {}", e))?;

    let mut interfaces: Vec<NetworkInterface> = adapters
        .iter()
        .filter_map(|adapter| {
            // Get IPv4 addresses, filtering out loopback and link-local
            let ipv4 = adapter
                .ip_addresses()
                .iter()
                .filter(|ip| ip.is_ipv4())
                .filter(|ip| !ip.is_loopback())
                .filter(|ip| {
                    // Filter out link-local addresses (169.254.x.x)
                    if let std::net::IpAddr::V4(v4) = ip {
                        !v4.is_link_local()
                    } else {
                        true
                    }
                })
                .next()
                .map(|ip| ip.to_string());

            // Only include adapters with at least one valid IPv4 address
            // or adapters that are currently connected (to show them as "disconnected")
            let is_up = adapter.oper_status() == ipconfig::OperStatus::IfOperStatusUp;

            // Skip adapters without IPv4 that are also not up (truly disconnected/unused)
            if ipv4.is_none() && !is_up {
                return None;
            }

            // Skip loopback adapters
            if adapter.if_type() == ipconfig::IfType::SoftwareLoopback {
                return None;
            }

            Some(NetworkInterface {
                id: adapter.adapter_name().to_string(),
                name: adapter.friendly_name().to_string(),
                ipv4,
                is_up,
            })
        })
        .collect();

    // Sort: connected interfaces first, then alphabetically by name
    interfaces.sort_by(|a, b| {
        match (a.is_up, b.is_up) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    Ok(interfaces)
}

#[cfg(not(target_os = "windows"))]
pub fn get_network_interfaces() -> Result<Vec<NetworkInterface>, String> {
    Ok(vec![])
}
