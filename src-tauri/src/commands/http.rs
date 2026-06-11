use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::io::{ErrorKind, Read, Write};
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::time::Duration;

const MAX_RESPONSE_BYTES: usize = 1024 * 1024;

pub(crate) struct HttpResponse {
    pub status: u16,
    pub body: String,
}

struct HttpUrl<'a> {
    host: &'a str,
    port: u16,
    path: String,
}

pub(crate) fn get_text(
    url: &str,
    params: &[(&str, &str)],
    local_addr: IpAddr,
    timeout: Duration,
    user_agent: &str,
) -> Result<HttpResponse, String> {
    let mut parsed = parse_http_url(url)?;
    append_query_params(&mut parsed.path, params);

    let mut stream = connect_bound(parsed.host, parsed.port, local_addr, timeout)?;
    let host_header = if parsed.port == 80 {
        parsed.host.to_string()
    } else {
        format!("{}:{}", parsed.host, parsed.port)
    };

    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: {}\r\nAccept: */*\r\nConnection: close\r\n\r\n",
        parsed.path, host_header, user_agent
    );

    stream
        .write_all(request.as_bytes())
        .map_err(|e| format!("发送请求失败: {}", e))?;

    let bytes = read_response(&mut stream)?;
    parse_response(&bytes)
}

fn parse_http_url(url: &str) -> Result<HttpUrl<'_>, String> {
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| format!("仅支持 HTTP URL: {}", url))?;
    let (authority, path) = match rest.find('/') {
        Some(index) => (&rest[..index], &rest[index..]),
        None => (rest, "/"),
    };

    if authority.is_empty() {
        return Err(format!("URL 缺少主机名: {}", url));
    }

    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) if !host.is_empty() => {
            let port = port
                .parse::<u16>()
                .map_err(|e| format!("URL 端口无效: {}", e))?;
            (host, port)
        }
        _ => (authority, 80),
    };

    Ok(HttpUrl {
        host,
        port,
        path: path.to_string(),
    })
}

fn append_query_params(path: &mut String, params: &[(&str, &str)]) {
    if params.is_empty() {
        return;
    }

    path.push(if path.contains('?') { '&' } else { '?' });
    for (index, (key, value)) in params.iter().enumerate() {
        if index > 0 {
            path.push('&');
        }
        path.push_str(&percent_encode_query_component(key));
        path.push('=');
        path.push_str(&percent_encode_query_component(value));
    }
}

fn percent_encode_query_component(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut encoded = String::with_capacity(value.len());

    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => {
                encoded.push('%');
                encoded.push(HEX[(byte >> 4) as usize] as char);
                encoded.push(HEX[(byte & 0x0F) as usize] as char);
            }
        }
    }

    encoded
}

fn connect_bound(
    host: &str,
    port: u16,
    local_addr: IpAddr,
    timeout: Duration,
) -> Result<std::net::TcpStream, String> {
    let local_socket_addr = SocketAddr::new(local_addr, 0);
    let remote_addrs = (host, port)
        .to_socket_addrs()
        .map_err(|e| format!("解析服务器地址失败: {}", e))?;
    let mut last_error = None;

    for remote_addr in remote_addrs {
        if remote_addr.is_ipv4() != local_socket_addr.is_ipv4() {
            continue;
        }

        match connect_one(remote_addr, local_socket_addr, timeout) {
            Ok(stream) => return Ok(stream),
            Err(err) => last_error = Some(err),
        }
    }

    Err(last_error.unwrap_or_else(|| "没有可用的同协议族服务器地址".to_string()))
}

fn connect_one(
    remote_addr: SocketAddr,
    local_addr: SocketAddr,
    timeout: Duration,
) -> Result<std::net::TcpStream, String> {
    let socket = Socket::new(
        Domain::for_address(remote_addr),
        Type::STREAM,
        Some(Protocol::TCP),
    )
    .map_err(|e| format!("创建 socket 失败: {}", e))?;

    socket
        .set_read_timeout(Some(timeout))
        .map_err(|e| format!("设置读取超时失败: {}", e))?;
    socket
        .set_write_timeout(Some(timeout))
        .map_err(|e| format!("设置写入超时失败: {}", e))?;
    socket
        .bind(&SockAddr::from(local_addr))
        .map_err(|e| format!("绑定本机地址失败: {}", e))?;
    socket
        .connect_timeout(&SockAddr::from(remote_addr), timeout)
        .map_err(|e| format!("连接服务器失败: {}", e))?;

    let stream = std::net::TcpStream::from(socket);
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|e| format!("设置读取超时失败: {}", e))?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|e| format!("设置写入超时失败: {}", e))?;

    Ok(stream)
}

fn read_response(stream: &mut std::net::TcpStream) -> Result<Vec<u8>, String> {
    let mut response = Vec::new();
    let mut buffer = [0; 8192];

    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => {
                response.extend_from_slice(&buffer[..count]);
                if response.len() > MAX_RESPONSE_BYTES {
                    return Err("响应过大".to_string());
                }
            }
            Err(error)
                if matches!(error.kind(), ErrorKind::TimedOut | ErrorKind::WouldBlock)
                    && !response.is_empty() =>
            {
                break;
            }
            Err(error) => return Err(format!("读取响应失败: {}", error)),
        }
    }

    if response.is_empty() {
        Err("服务器未返回响应".to_string())
    } else {
        Ok(response)
    }
}

fn parse_response(bytes: &[u8]) -> Result<HttpResponse, String> {
    let (header_end, body_start) =
        find_header_boundary(bytes).ok_or_else(|| "HTTP 响应头不完整".to_string())?;
    let headers = String::from_utf8_lossy(&bytes[..header_end]);
    let mut lines = headers.lines();
    let status_line = lines
        .next()
        .ok_or_else(|| "HTTP 响应缺少状态行".to_string())?;
    let status = status_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| "HTTP 响应状态行无效".to_string())?
        .parse::<u16>()
        .map_err(|e| format!("HTTP 状态码无效: {}", e))?;

    let chunked = lines
        .filter_map(|line| line.split_once(':'))
        .any(|(key, value)| {
            key.trim().eq_ignore_ascii_case("transfer-encoding")
                && value.to_ascii_lowercase().contains("chunked")
        });

    let body_bytes = if chunked {
        decode_chunked_body(&bytes[body_start..])?
    } else {
        bytes[body_start..].to_vec()
    };

    Ok(HttpResponse {
        status,
        body: String::from_utf8_lossy(&body_bytes).to_string(),
    })
}

fn find_header_boundary(bytes: &[u8]) -> Option<(usize, usize)> {
    bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| (index, index + 4))
        .or_else(|| {
            bytes
                .windows(2)
                .position(|window| window == b"\n\n")
                .map(|index| (index, index + 2))
        })
}

fn decode_chunked_body(mut body: &[u8]) -> Result<Vec<u8>, String> {
    let mut decoded = Vec::new();

    loop {
        let line_end = body
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or_else(|| "chunked 响应缺少分块长度".to_string())?;
        let size_line = String::from_utf8_lossy(&body[..line_end]);
        let size_hex = size_line.split(';').next().unwrap_or("").trim();
        let size = usize::from_str_radix(size_hex, 16)
            .map_err(|e| format!("chunked 分块长度无效: {}", e))?;
        body = &body[line_end + 2..];

        if size == 0 {
            break;
        }

        if body.len() < size + 2 {
            return Err("chunked 响应体不完整".to_string());
        }

        decoded.extend_from_slice(&body[..size]);
        body = &body[size + 2..];
    }

    Ok(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_encoding_uses_utf8_percent_encoding() {
        assert_eq!(
            percent_encode_query_component("user@校园 A"),
            "user%40%E6%A0%A1%E5%9B%AD%20A"
        );
    }

    #[test]
    fn parses_plain_http_response() {
        let response = parse_response(b"HTTP/1.1 200 OK\r\nContent-Length: 7\r\n\r\nsuccess")
            .expect("response should parse");

        assert_eq!(response.status, 200);
        assert_eq!(response.body, "success");
    }

    #[test]
    fn parses_chunked_http_response() {
        let response = parse_response(
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n4\r\nsucc\r\n3\r\ness\r\n0\r\n\r\n",
        )
        .expect("response should parse");

        assert_eq!(response.status, 200);
        assert_eq!(response.body, "success");
    }
}
