//! # DNS Optimizer
//! A command-line tool for optimizing DNS settings on macOS systems by testing various DNS providers
//! and automatically configuring the fastest one.
//!
//! ## Features
//! - Automatic DNS server detection
//! - Latency testing for multiple DNS providers
//! - Automatic configuration of the fastest DNS server

use regex::Regex;
use std::{
    process::Command,
    thread,
    time::{Duration, Instant},
};

/// Logs a message with a timestamp prefix.
///
/// # Arguments
/// * `message` - The message to be logged
fn log(message: &str) {
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    println!("[{}] {}", timestamp, message);
}

/// Retrieves the current DNS server configuration from the system.
///
/// Uses the `scutil` command to query DNS settings and parses the output
/// to extract nameserver IP addresses.
fn get_current_dns() -> Vec<String> {
    let output = Command::new("scutil")
        .arg("--dns")
        .output()
        .expect("Failed to execute scutil command");

    let output_str = String::from_utf8_lossy(&output.stdout);
    let first_section = output_str
        .split("DNS configuration (for scoped queries)")
        .next()
        .unwrap();

    let ip_pattern = Regex::new(r"nameserver\[\d\]\s*:\s*([^\s]+)").unwrap();

    first_section
        .lines()
        .filter(|line| line.trim().starts_with("nameserver"))
        .filter_map(|line| {
            ip_pattern
                .captures(line)
                .and_then(|cap| cap.get(1))
                .map(|ip| ip.as_str().to_string())
        })
        .collect()
}

/// Sets the DNS servers for the Wi-Fi interface.
///
/// * Includes a 2-second delay after setting DNS to allow changes to take effect
/// * Only affects the Wi-Fi interface
fn set_dns(dns: &str) {
    log(&format!("Setting DNS servers to: {}", dns));
    Command::new("networksetup")
        .args(["-setdnsservers", "Wi-Fi", dns])
        .output()
        .expect(&format!("Failed to set DNS to {}", dns));

    // Give some time for DNS changes to take effect
    thread::sleep(Duration::from_secs(2));
    log("DNS settings applied");
}

/// Sets DNS configuration to automatic (DHCP) mode.
///
/// This removes any manually configured DNS servers and allows
/// the system to obtain DNS settings automatically from DHCP.
fn set_dns_automatic() {
    log("Setting DNS to automatic (empty)");
    Command::new("networksetup")
        .args(["-setdnsservers", "Wi-Fi", "empty"])
        .output()
        .expect("Failed to set DNS to automatic");

    thread::sleep(Duration::from_secs(2));
    log("DNS set to automatic mode");
}

/// Measures the latency to a DNS server using ping.
fn measure_latency(dns: &str) -> Duration {
    log(&format!("Testing latency for {}", dns));
    let start = Instant::now();
    Command::new("ping")
        .args(["-c", "3", dns])
        .output()
        .expect("Failed to ping DNS");
    let latency = start.elapsed() / 3;
    log(&format!("Latency for {}: {:?}", dns, latency));
    latency
}

/// Prints the current DNS configuration.
///
/// Retrieves and displays the current DNS servers configured on the system.
/// If no DNS servers are configured (empty list), indicates that DNS is set
/// to automatic (DHCP) mode.
fn print_current_dns() {
    let current_dns = get_current_dns();
    log("Current DNS servers:");
    if current_dns.is_empty() {
        log("  • Automatic (DHCP)");
    } else {
        for dns in current_dns {
            log(&format!("  • {}", dns));
        }
    }
}

/// The optimization process follows these steps:
/// 1. Display current DNS configuration
/// 2. Reset to automatic DNS
/// 3. Test latency of various DNS servers
/// 4. Print test results
/// 5. Configure the fastest DNS server
/// 6. Display final DNS configuration
///
/// # Notes
/// * The process tests multiple DNS providers including Cloudflare, Google, Quad9, etc.
/// * Each provider's primary and secondary servers are tested
/// * Results are sorted by latency
/// * The fastest DNS server is automatically configured
fn main() {
    log("=== DNS Optimization Tool ===");

    // 1. Show current DNS
    log("\nChecking current DNS configuration...");
    print_current_dns();

    // 2. Set to automatic
    log("\nResetting to automatic DNS...");
    set_dns_automatic();

    // 3. Test various DNS servers
    let dns_servers = [
        // Cloudflare - Known for speed and privacy
        ("Cloudflare Primary", "1.1.1.1"),
        ("Cloudflare Secondary", "1.0.0.1"),
        // Google - Most popular, highly reliable
        ("Google Primary", "8.8.8.8"),
        ("Google Secondary", "8.8.4.4"),
        // Quad9 - Security focused, blocks malicious domains
        ("Quad9 Primary", "9.9.9.9"),
        ("Quad9 Secondary", "149.112.112.112"),
        // OpenDNS - Cisco owned, extensive filtering
        ("OpenDNS Primary", "208.67.222.222"),
        ("OpenDNS Secondary", "208.67.220.220"),
        // AdGuard - Ad blocking, no logging
        ("AdGuard Primary", "94.140.14.14"),
        ("AdGuard Secondary", "94.140.15.15"),
        // CleanBrowsing - Family friendly filtering
        ("CleanBrowsing Primary", "185.228.168.9"),
        ("CleanBrowsing Secondary", "185.228.169.9"),
        // Level3/CenturyLink - Enterprise grade
        ("Level3 Primary", "4.2.2.1"),
        ("Level3 Secondary", "4.2.2.2"),
        // Comodo Secure - Security focused
        ("Comodo Primary", "8.26.56.26"),
        ("Comodo Secondary", "8.20.247.20"),
        // Verisign - Enterprise reliability
        ("Verisign Primary", "64.6.64.6"),
        ("Verisign Secondary", "64.6.65.6"),
        // NextDNS - Cloud-based, customizable
        ("NextDNS", "45.90.28.167"),
    ];

    log("\nStarting DNS latency tests...");
    let mut latencies: Vec<_> = dns_servers
        .iter()
        .map(|(name, ip)| {
            let latency = measure_latency(ip);
            (name, ip, latency)
        })
        .collect();

    latencies.sort_by_key(|&(_, _, latency)| latency);

    // 4. Print results
    log("\nLatency Test Results:");
    println!("{:-<50}", "");
    for (name, ip, latency) in &latencies {
        println!("{:12} ({:10}) : {:.2?}", name, ip, latency);
    }
    println!("{:-<50}", "");

    // 5. Set to fastest
    let (fastest_name, fastest_ip, fastest_latency) = latencies[0];
    log(&format!(
        "\nSetting DNS to fastest server: {} ({}) with latency {:?}",
        fastest_name, fastest_ip, fastest_latency
    ));
    set_dns(fastest_ip);

    // 6. Show final DNS configuration
    log("\nFinal DNS configuration:");
    print_current_dns();

    log("\nDNS optimization completed!");
}
