//! Client of the node's loopback control API: reads the read token the node wrote, asks
//! `GET /v1/worker`, and reads the HTTP answer. Only 127.0.0.1 is ever contacted.

use crate::node_process::{CONTROL_PORT, NodeError};
use serde_json::Value;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::Path;
use std::time::Duration;

const IO_TIMEOUT: Duration = Duration::from_secs(3);

/// The read token from the node's token file (`scope1,scope2:secret` lines).
pub fn read_token(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    text.lines().find_map(|line| {
        let (scopes, secret) = line.trim().split_once(':')?;
        let has_read = scopes.split(',').any(|scope| scope.trim() == "read");
        (has_read && !secret.is_empty()).then(|| secret.to_string())
    })
}

/// `GET /v1/worker` on the node, with the read token.
pub fn worker_report(token: &str) -> Result<Value, NodeError> {
    let raw = http_get(CONTROL_PORT, "/v1/worker", token)?;
    parse_answer(&raw)
}

fn http_get(port: u16, path: &str, token: &str) -> Result<String, NodeError> {
    let address = ("127.0.0.1", port)
        .to_socket_addrs()
        .ok()
        .and_then(|mut addresses| addresses.next())
        .ok_or(NodeError::Unreachable)?;
    let mut stream =
        TcpStream::connect_timeout(&address, IO_TIMEOUT).map_err(|_| NodeError::Unreachable)?;
    stream
        .set_read_timeout(Some(IO_TIMEOUT))
        .map_err(|_| NodeError::Unreachable)?;
    stream
        .set_write_timeout(Some(IO_TIMEOUT))
        .map_err(|_| NodeError::Unreachable)?;
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer {token}\r\nConnection: close\r\n\r\n"
    )
    .map_err(|_| NodeError::Unreachable)?;
    let mut raw = Vec::new();
    stream
        .read_to_end(&mut raw)
        .map_err(|_| NodeError::Unreachable)?;
    Ok(String::from_utf8_lossy(&raw).into_owned())
}

/// Reads an HTTP/1.1 answer: status, then JSON body (plain or chunked).
pub fn parse_answer(raw: &str) -> Result<Value, NodeError> {
    let (head, body) = raw.split_once("\r\n\r\n").ok_or(NodeError::BadAnswer)?;
    let status = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or(NodeError::BadAnswer)?;
    match status {
        200 => {
            let chunked = head
                .lines()
                .any(|line| line.eq_ignore_ascii_case("transfer-encoding: chunked"));
            let json = if chunked {
                decode_chunked(body)?
            } else {
                body.to_string()
            };
            serde_json::from_str(&json).map_err(|_| NodeError::BadAnswer)
        }
        401 => Err(NodeError::Unauthorized),
        _ => Err(NodeError::BadAnswer),
    }
}

/// Decodes a chunked transfer body.
pub fn decode_chunked(body: &str) -> Result<String, NodeError> {
    let mut out = String::new();
    let mut rest = body;
    loop {
        let (size_line, after) = rest.split_once("\r\n").ok_or(NodeError::BadAnswer)?;
        let size = usize::from_str_radix(size_line.split(';').next().unwrap_or("").trim(), 16)
            .map_err(|_| NodeError::BadAnswer)?;
        if size == 0 {
            return Ok(out);
        }
        let chunk = after.get(..size).ok_or(NodeError::BadAnswer)?;
        out.push_str(chunk);
        rest = after.get(size + 2..).ok_or(NodeError::BadAnswer)?;
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "tests fail loudly on purpose")]
mod tests {
    use super::*;

    #[test]
    fn the_read_token_is_picked_from_the_token_file() {
        let dir = tempfile::tempdir().expect("dir");
        let path = dir.path().join("control.token");
        fs::write(
            &path,
            "# comment\nread,submit:writer-secret\nread:reader-secret-0123456789abcdef\n",
        )
        .expect("write");
        assert_eq!(read_token(&path).as_deref(), Some("writer-secret"));
        fs::write(&path, "submit:only-submit\n").expect("write");
        assert_eq!(read_token(&path), None);
    }

    #[test]
    fn answers_are_read_by_status_and_body() {
        let ok = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"jobs_touched\":2}";
        assert_eq!(parse_answer(ok).expect("json")["jobs_touched"], 2);
        let refused = "HTTP/1.1 401 Unauthorized\r\n\r\n";
        assert_eq!(parse_answer(refused), Err(NodeError::Unauthorized));
        assert_eq!(parse_answer("garbage"), Err(NodeError::BadAnswer));
    }

    #[test]
    fn chunked_answers_are_joined() {
        let chunked = "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n7\r\n{\"a\":1,\r\n6\r\n\"b\":2}\r\n0\r\n\r\n";
        assert_eq!(parse_answer(chunked).expect("json")["b"], 2);
    }
}
