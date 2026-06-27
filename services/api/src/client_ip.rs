//! client_ip.rs — Trusted-proxy IP extraction with spoofing prevention.
//!
//! Only trusts X-Forwarded-For / X-Real-IP when the direct peer address
//! falls within a configured set of trusted CIDR ranges.
//! If the peer is untrusted, the peer address is used directly.

use std::net::IpAddr;
use std::str::FromStr;

/// Returns true if `ip` falls within any of the provided CIDR strings.
/// Supports both IPv4 (e.g. "10.0.0.0/8") and plain IPs as /32 or /128.
pub fn is_trusted_proxy(ip: &IpAddr, trusted_cidrs: &[String]) -> bool {
    for cidr in trusted_cidrs {
        if let Some((network, prefix)) = cidr.split_once('/') {
            if let (Ok(network_ip), Ok(prefix_len)) =
                (IpAddr::from_str(network), prefix.parse::<u8>())
            {
                if ip_in_cidr(ip, &network_ip, prefix_len) {
                    return true;
                }
            }
        } else if let Ok(plain) = IpAddr::from_str(cidr) {
            if ip == &plain {
                return true;
            }
        }
    }
    false
}

fn ip_in_cidr(ip: &IpAddr, network: &IpAddr, prefix_len: u8) -> bool {
    match (ip, network) {
        (IpAddr::V4(ip4), IpAddr::V4(net4)) => {
            if prefix_len > 32 {
                return false;
            }
            let mask = if prefix_len == 0 { 0u32 } else { !0u32 << (32 - prefix_len) };
            (u32::from(*ip4) & mask) == (u32::from(*net4) & mask)
        }
        (IpAddr::V6(ip6), IpAddr::V6(net6)) => {
            if prefix_len > 128 {
                return false;
            }
            let ip_bits = u128::from(*ip6);
            let net_bits = u128::from(*net6);
            let mask = if prefix_len == 0 { 0u128 } else { !0u128 << (128 - prefix_len) };
            (ip_bits & mask) == (net_bits & mask)
        }
        _ => false,
    }
}

/// Extract the real client IP from request headers, respecting trusted proxies.
///
/// - If `peer_addr` is within `trusted_cidrs`, use the first IP in
///   `x_forwarded_for`, falling back to `x_real_ip`, then `peer_addr`.
/// - If `peer_addr` is NOT trusted, return `peer_addr` directly (spoofing prevention).
pub fn extract_client_ip(
    peer_addr: &IpAddr,
    x_forwarded_for: Option<&str>,
    x_real_ip: Option<&str>,
    trusted_cidrs: &[String],
) -> IpAddr {
    if !is_trusted_proxy(peer_addr, trusted_cidrs) {
        return *peer_addr;
    }
    if let Some(xff) = x_forwarded_for {
        if let Some(first) = xff.split(',').next() {
            if let Ok(ip) = IpAddr::from_str(first.trim()) {
                return ip;
            }
        }
    }
    if let Some(xri) = x_real_ip {
        if let Ok(ip) = IpAddr::from_str(xri.trim()) {
            return ip;
        }
    }
    *peer_addr
}

/// Load trusted CIDR list from TRUSTED_PROXY_CIDRS env var (comma-separated).
/// Defaults to RFC 1918 private ranges + loopback if unset.
pub fn trusted_cidrs_from_env() -> Vec<String> {
    std::env::var("TRUSTED_PROXY_CIDRS")
        .ok()
        .map(|v| v.split(',').map(|s| s.trim().to_owned()).collect())
        .unwrap_or_else(|| {
            vec![
                "127.0.0.0/8".to_owned(),
                "10.0.0.0/8".to_owned(),
                "172.16.0.0/12".to_owned(),
                "192.168.0.0/16".to_owned(),
                "::1/128".to_owned(),
            ]
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    fn trusted() -> Vec<String> {
        vec!["10.0.0.0/8".to_owned(), "127.0.0.1/32".to_owned()]
    }

    #[test]
    fn trusted_proxy_xff_used() {
        let peer = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        let ip = extract_client_ip(&peer, Some("203.0.113.5, 10.0.0.1"), None, &trusted());
        assert_eq!(ip, IpAddr::V4(Ipv4Addr::new(203, 0, 113, 5)));
    }

    #[test]
    fn untrusted_proxy_peer_used_directly() {
        let peer = IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4));
        let ip = extract_client_ip(&peer, Some("203.0.113.5"), None, &trusted());
        assert_eq!(ip, peer);
    }

    #[test]
    fn falls_back_to_x_real_ip_when_no_xff() {
        let peer = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2));
        let ip = extract_client_ip(&peer, None, Some("198.51.100.1"), &trusted());
        assert_eq!(ip, IpAddr::V4(Ipv4Addr::new(198, 51, 100, 1)));
    }

    #[test]
    fn falls_back_to_peer_when_all_headers_absent() {
        let peer = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 3));
        let ip = extract_client_ip(&peer, None, None, &trusted());
        assert_eq!(ip, peer);
    }

    #[test]
    fn ipv6_loopback_is_trusted() {
        let cidrs = vec!["::1/128".to_owned()];
        let peer = IpAddr::V6(Ipv6Addr::LOCALHOST);
        assert!(is_trusted_proxy(&peer, &cidrs));
    }

    #[test]
    fn invalid_xff_falls_through_to_peer() {
        let peer = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 4));
        let ip = extract_client_ip(&peer, Some("not-an-ip"), None, &trusted());
        assert_eq!(ip, peer);
    }
}
