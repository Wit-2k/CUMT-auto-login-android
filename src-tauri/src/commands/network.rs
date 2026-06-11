use crate::commands::auth::get_local_ip;
use crate::commands::http;
use std::net::IpAddr;
use std::time::Duration;

#[derive(Clone, Copy)]
enum ProbeExpectation {
    NoContent,
    ExactBody(&'static str),
}

#[derive(Clone, Copy)]
struct ConnectivityProbe {
    url: &'static str,
    expectation: ProbeExpectation,
}

const CONNECTIVITY_PROBES: &[ConnectivityProbe] = &[
    ConnectivityProbe {
        url: "http://connect.rom.miui.com/generate_204",
        expectation: ProbeExpectation::NoContent,
    },
    ConnectivityProbe {
        url: "http://www.gstatic.com/generate_204",
        expectation: ProbeExpectation::NoContent,
    },
    ConnectivityProbe {
        url: "http://detectportal.firefox.com/success.txt",
        expectation: ProbeExpectation::ExactBody("success"),
    },
];

fn is_success_response(expectation: ProbeExpectation, status: u16, body: &str) -> bool {
    match expectation {
        ProbeExpectation::NoContent => status == 204 && body.trim().is_empty(),
        ProbeExpectation::ExactBody(expected) => {
            (200..300).contains(&status) && body.trim() == expected
        }
    }
}

/// 检查是否已连接到互联网
#[tauri::command]
pub async fn check_online() -> bool {
    tauri::async_runtime::spawn_blocking(check_online_blocking)
        .await
        .unwrap_or(false)
}

fn check_online_blocking() -> bool {
    let local_ip = match get_local_ip() {
        Ok(ip) => ip,
        Err(_) => return false,
    };
    let local_addr = match local_ip.parse::<IpAddr>() {
        Ok(addr) => addr,
        Err(_) => return false,
    };

    for probe in CONNECTIVITY_PROBES {
        if let Ok(response) = http::get_text(
            probe.url,
            &[],
            local_addr,
            Duration::from_secs(2),
            "Mozilla/5.0",
        ) {
            if is_success_response(probe.expectation, response.status, &response.body) {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_exact_success_body() {
        assert!(is_success_response(
            ProbeExpectation::ExactBody("success"),
            200,
            "success\n"
        ));
    }

    #[test]
    fn accepts_generate_204_no_content() {
        assert!(is_success_response(ProbeExpectation::NoContent, 204, ""));
    }

    #[test]
    fn rejects_captive_portal_html_even_with_200() {
        assert!(!is_success_response(
            ProbeExpectation::ExactBody("success"),
            200,
            "<html><title>Portal Login</title></html>"
        ));
    }

    #[test]
    fn rejects_captive_portal_body_for_generate_204() {
        assert!(!is_success_response(
            ProbeExpectation::NoContent,
            200,
            "<html><title>Portal Login</title></html>"
        ));
    }

    #[test]
    fn rejects_non_success_status_for_exact_body() {
        assert!(!is_success_response(
            ProbeExpectation::ExactBody("success"),
            302,
            "success"
        ));
    }
}
