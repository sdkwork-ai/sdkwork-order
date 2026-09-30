use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

use sdkwork_contract_service::{CommerceLedgerBusinessType, CommerceMoney};
use sdkwork_order_integration_account::HttpAccountPointsCreditAdapter;
use sdkwork_order_service::{
    AccountValueAssetCode, AccountValueLedgerCommand, AccountValueLedgerPort,
};
use serde_json::{json, Value};

#[tokio::test]
async fn http_account_value_hold_posts_to_token_bank_hold_endpoint() {
    let server = OneShotHttpServer::spawn(201, hold_response("hold-token-bank-1", None));
    let adapter = HttpAccountPointsCreditAdapter::new(server.origin(), Some("test-token".into())).expect("adapter builds");

    let outcome = adapter
        .apply_account_value_ledger_command(
            AccountValueLedgerCommand::hold(
                "tenant-1",
                Some("org-1"),
                "user-1",
                AccountValueAssetCode::TokenBank,
                CommerceMoney::new("1200").expect("amount"),
                "TOKEN_BANK",
                CommerceLedgerBusinessType::TOKEN_BANK_HOLD,
                "refund-request-1",
                "req-hold-1",
                "idem-hold-1",
            )
            .expect("hold command"),
        )
        .await
        .expect("hold request");

    let request = server.recorded_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/backend/v3/api/token_bank/holds");
    assert_eq!(request.authorization.as_deref(), Some("Bearer test-token"));
    assert_eq!(request.body["tenantId"], "tenant-1");
    assert_eq!(request.body["organizationId"], "org-1");
    assert_eq!(request.body["ownerUserId"], "user-1");
    assert_eq!(request.body["assetType"], "token_bank");
    assert_eq!(request.body["amount"], "1200");
    assert_eq!(request.body["businessNo"], "refund-request-1");
    assert_eq!(request.body["sourceType"], "commerce_order_request");
    assert!(request.body["sourceId"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    assert_eq!(request.body["requestNo"], "req-hold-1");
    assert_eq!(request.body["idempotencyKey"], "idem-hold-1");
    assert_eq!(
        outcome.account_effect_reference_id.as_deref(),
        Some("hold-token-bank-1")
    );
}

#[tokio::test]
async fn http_account_value_hold_settle_posts_to_hold_settle_endpoint() {
    let server =
        OneShotHttpServer::spawn(200, hold_response("hold-token-bank-1", Some("ledger-1")));
    let adapter = HttpAccountPointsCreditAdapter::new(server.origin(), Some("test-token".into())).expect("adapter builds");

    let outcome = adapter
        .apply_account_value_ledger_command(
            AccountValueLedgerCommand::hold_settle(
                "tenant-1",
                Some("org-1"),
                "user-1",
                AccountValueAssetCode::TokenBank,
                CommerceMoney::new("1200").expect("amount"),
                "TOKEN_BANK",
                CommerceLedgerBusinessType::TOKEN_BANK_REVERSAL,
                "hold-token-bank-1",
                "req-settle-1",
                "idem-settle-1",
            )
            .expect("settle command"),
        )
        .await
        .expect("settle request");

    let request = server.recorded_request();
    assert_eq!(request.method, "POST");
    assert_eq!(
        request.path,
        "/backend/v3/api/token_bank/holds/hold-token-bank-1/settle"
    );
    assert_eq!(request.body["tenantId"], "tenant-1");
    assert_eq!(
        request.body["businessType"],
        CommerceLedgerBusinessType::TOKEN_BANK_REVERSAL
    );
    assert_eq!(
        request.body["transactionNo"],
        "token_bank_reversal:hold-token-bank-1"
    );
    assert_eq!(request.body["requestNo"], "req-settle-1");
    assert_eq!(request.body["idempotencyKey"], "idem-settle-1");
    assert_eq!(outcome.ledger_entry_id.as_deref(), Some("ledger-1"));
    assert_eq!(
        outcome.account_effect_reference_id.as_deref(),
        Some("hold-token-bank-1")
    );
}

#[tokio::test]
async fn http_account_value_hold_release_posts_to_wallet_hold_release_endpoint() {
    let server = OneShotHttpServer::spawn(200, hold_response("cash-hold-1", None));
    let adapter = HttpAccountPointsCreditAdapter::new(server.origin(), Some("test-token".into())).expect("adapter builds");

    let outcome = adapter
        .apply_account_value_ledger_command(
            AccountValueLedgerCommand::hold_release(
                "tenant-1",
                Some("org-1"),
                "user-1",
                AccountValueAssetCode::Cash,
                CommerceMoney::new("1200").expect("amount"),
                "CNY",
                "cash_withdrawal",
                "cash-hold-1",
                "req-release-1",
                "idem-release-1",
            )
            .expect("release command"),
        )
        .await
        .expect("release request");

    let request = server.recorded_request();
    assert_eq!(request.method, "POST");
    assert_eq!(
        request.path,
        "/backend/v3/api/wallet/holds/cash-hold-1/release"
    );
    assert_eq!(request.body["tenantId"], "tenant-1");
    assert_eq!(request.body["requestNo"], "req-release-1");
    assert_eq!(request.body["idempotencyKey"], "idem-release-1");
    assert_eq!(
        outcome.account_effect_reference_id.as_deref(),
        Some("cash-hold-1")
    );
}

struct OneShotHttpServer {
    origin: String,
    handle: thread::JoinHandle<RecordedRequest>,
}

struct RecordedRequest {
    method: String,
    path: String,
    authorization: Option<String>,
    body: Value,
}

impl OneShotHttpServer {
    fn spawn(status_code: u16, body: Value) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let origin = format!("http://{}", listener.local_addr().expect("local addr"));
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let raw_request = read_http_request(&mut stream);
            let recorded = parse_recorded_request(&raw_request);
            let response_body = serde_json::to_string(&body).expect("response body");
            let reason = if status_code == 201 { "Created" } else { "OK" };
            let response = format!(
                "HTTP/1.1 {status_code} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                response_body.len(),
                response_body
            );
            stream
                .write_all(response.as_bytes())
                .expect("write response");
            recorded
        });
        Self { origin, handle }
    }

    fn origin(&self) -> String {
        self.origin.clone()
    }

    fn recorded_request(self) -> RecordedRequest {
        self.handle.join().expect("server thread")
    }
}

fn hold_response(hold_id: &str, ledger_entry_id: Option<&str>) -> Value {
    json!({
        "code": 0,
        "traceId": "trace-1",
        "data": {
            "item": {
                "accepted": true,
                "replayed": false,
                "hold": {
                    "uuid": hold_id
                },
                "ledgerEntry": ledger_entry_id.map(|id| json!({ "id": id }))
            }
        }
    })
}

fn read_http_request(stream: &mut TcpStream) -> String {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    let mut header_end = None;
    let mut content_length = None;

    loop {
        let read = stream.read(&mut buffer).expect("read request");
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
        if header_end.is_none() {
            header_end = find_header_end(&bytes);
            if let Some(end) = header_end {
                let headers = String::from_utf8_lossy(&bytes[..end]);
                content_length = parse_content_length(&headers);
            }
        }
        if let (Some(end), Some(length)) = (header_end, content_length) {
            if bytes.len() >= end + 4 + length {
                break;
            }
        }
    }

    String::from_utf8(bytes).expect("utf8 request")
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

fn parse_content_length(headers: &str) -> Option<usize> {
    headers.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name.eq_ignore_ascii_case("content-length") {
            value.trim().parse::<usize>().ok()
        } else {
            None
        }
    })
}

fn parse_recorded_request(raw_request: &str) -> RecordedRequest {
    let (head, body_text) = raw_request
        .split_once("\r\n\r\n")
        .expect("request separator");
    let mut lines = head.lines();
    let request_line = lines.next().expect("request line");
    let mut request_line_parts = request_line.split_whitespace();
    let method = request_line_parts.next().expect("method").to_owned();
    let path = request_line_parts.next().expect("path").to_owned();
    let authorization = lines.find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name.eq_ignore_ascii_case("authorization") {
            Some(value.trim().to_owned())
        } else {
            None
        }
    });
    let body = serde_json::from_str(body_text).expect("json request body");
    RecordedRequest {
        method,
        path,
        authorization,
        body,
    }
}
