use hermetica::{Client, Params};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn reply(status: u16) -> ResponseTemplate {
    ResponseTemplate::new(status).insert_header("Retry-After", "0").set_body_json(json!({ "ok": true }))
}

#[tokio::test]
async fn retries_with_the_same_idempotency_key() {
    let server = MockServer::start().await;
    Mock::given(method("POST")).and(path("/v1/profiles")).respond_with(reply(503)).up_to_n_times(1).mount(&server).await;
    Mock::given(method("POST")).and(path("/v1/profiles")).respond_with(reply(429)).up_to_n_times(1).mount(&server).await;
    Mock::given(method("POST")).and(path("/v1/profiles")).respond_with(reply(200)).mount(&server).await;
    let hm = Client::new("hm_sk_test", server.uri());
    let out = hm.post_v1_profiles(Params::new().body(json!({ "email": "a@b.co" }))).await.unwrap();
    assert_eq!(out, json!({ "ok": true }));
    let seen = server.received_requests().await.unwrap();
    assert_eq!(seen.len(), 3);
    let keys: Vec<_> = seen.iter().map(|r| r.headers.get("Idempotency-Key").unwrap().to_str().unwrap().to_string()).collect();
    assert!(!keys[0].is_empty() && keys.iter().all(|k| k == &keys[0]));
    assert!(seen.iter().all(|r| r.body == br#"{"email":"a@b.co"}"#));
}

#[tokio::test]
async fn does_not_retry_4xx_and_reads_the_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/healthz"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({ "error": { "code": "not_found", "message": "nope" } })))
        .mount(&server).await;
    let hm = Client::new("hm_sk_test", server.uri());
    let e = hm.get_healthz(Params::new()).await.unwrap_err();
    assert_eq!((e.status, e.code.as_str(), e.message.as_str()), (404, "not_found", "nope"));
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn gives_up_after_max_retries() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).respond_with(reply(502)).mount(&server).await;
    let mut hm = Client::new("hm_sk_test", server.uri());
    hm.max_retries = 1;
    assert_eq!(hm.get_healthz(Params::new()).await.unwrap_err().status, 502);
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn follows_cursors() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/v1/profiles")).and(wiremock::matchers::query_param("cursor", "c2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "items": [2], "nextCursor": null }))).mount(&server).await;
    Mock::given(method("GET")).and(path("/v1/profiles"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "items": [1], "nextCursor": "c2" }))).mount(&server).await;
    let hm = Client::new("hm_sk_test", server.uri());
    assert_eq!(hm.list_all("GET", "/v1/profiles", Params::new()).await.unwrap(), vec![json!(1), json!(2)]);
}
