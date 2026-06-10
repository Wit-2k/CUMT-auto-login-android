use crate::commands::auth::get_local_ip;
use reqwest::{Client, StatusCode};
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

fn is_success_response(expectation: ProbeExpectation, status: StatusCode, body: &str) -> bool {
    match expectation {
        ProbeExpectation::NoContent => status == StatusCode::NO_CONTENT && body.trim().is_empty(),
        ProbeExpectation::ExactBody(expected) => status.is_success() && body.trim() == expected,
    }
}

/// 检查是否已连接到互联网
#[tauri::command]
pub async fn check_online() -> bool {
    let local_ip = match get_local_ip() {
        Ok(ip) => ip,
        Err(_) => return false,
    };
    let local_addr = match local_ip.parse::<IpAddr>() {
        Ok(addr) => addr,
        Err(_) => return false,
    };

    let client = match Client::builder()
        .timeout(Duration::from_secs(2))
        .local_address(local_addr)
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };

    for probe in CONNECTIVITY_PROBES {
        if let Ok(resp) = client.get(probe.url).send().await {
            let status = resp.status();
            if let Ok(body) = resp.text().await {
                if is_success_response(probe.expectation, status, &body) {
                    return true;
                }
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
            StatusCode::OK,
            "success\n"
        ));
    }

    #[test]
    fn accepts_generate_204_no_content() {
        assert!(is_success_response(
            ProbeExpectation::NoContent,
            StatusCode::NO_CONTENT,
            ""
        ));
    }

    #[test]
    fn rejects_captive_portal_html_even_with_200() {
        assert!(!is_success_response(
            ProbeExpectation::ExactBody("success"),
            StatusCode::OK,
            "<html><title>Portal Login</title></html>"
        ));
    }

    #[test]
    fn rejects_captive_portal_body_for_generate_204() {
        assert!(!is_success_response(
            ProbeExpectation::NoContent,
            StatusCode::OK,
            "<html><title>Portal Login</title></html>"
        ));
    }

    #[test]
    fn rejects_non_success_status_for_exact_body() {
        assert!(!is_success_response(
            ProbeExpectation::ExactBody("success"),
            StatusCode::FOUND,
            "success"
        ));
    }
}
