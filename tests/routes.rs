use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use srvcs_majority::{health, router, telemetry};
use tower::ServiceExt;

async fn status_of(uri: &str) -> StatusCode {
    let app = router(telemetry::metrics_handle_for_tests());
    app.oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap()
        .status()
}

/// POST `{"values": ...}` to `/` and return (status, parsed JSON response).
async fn eval(values: Value) -> (StatusCode, Value) {
    let app = router(telemetry::metrics_handle_for_tests());
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "values": values }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

// --- Standard srvcs service surface ---

#[tokio::test]
async fn index_ok() {
    assert_eq!(status_of("/").await, StatusCode::OK);
}

#[tokio::test]
async fn healthz_ok() {
    assert_eq!(status_of("/healthz").await, StatusCode::OK);
}

#[tokio::test]
async fn readyz_reflects_state() {
    health::set_ready(true);
    assert_eq!(status_of("/readyz").await, StatusCode::OK);
}

#[tokio::test]
async fn metrics_ok() {
    assert_eq!(status_of("/metrics").await, StatusCode::OK);
}

#[tokio::test]
async fn openapi_ok() {
    assert_eq!(status_of("/openapi.json").await, StatusCode::OK);
}

// --- Truth-table cases for strict majority ---

#[tokio::test]
async fn strict_majority_is_true() {
    // 2 of 3 true is a strict majority.
    let (status, body) = eval(json!([true, true, false])).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"], true);
    assert_eq!(body["values"], json!([true, true, false]));
}

#[tokio::test]
async fn singleton_true_is_majority() {
    let (status, body) = eval(json!([true])).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"], true);
}

#[tokio::test]
async fn one_true_among_three_is_not_majority() {
    // 1 of 3 true: a minority, not a majority.
    let (status, body) = eval(json!([true, false, false])).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"], false);
}

#[tokio::test]
async fn tie_is_not_a_majority() {
    // 1 of 2 true is a tie, which is not strictly more than half.
    let (status, body) = eval(json!([true, false])).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"], false);
}

#[tokio::test]
async fn even_split_is_not_a_majority() {
    let (status, body) = eval(json!([true, true, false, false])).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"], false);
}

#[tokio::test]
async fn all_false_is_false() {
    let (status, body) = eval(json!([false, false, false])).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"], false);
}

#[tokio::test]
async fn empty_list_is_false() {
    let (status, body) = eval(json!([])).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"], false);
    assert_eq!(body["values"], json!([]));
}

// --- Error / edge cases ---

#[tokio::test]
async fn non_boolean_element_is_422() {
    let (status, body) = eval(json!([true, "nope", false])).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"], "values must be booleans");
}

#[tokio::test]
async fn number_element_is_422() {
    let (status, body) = eval(json!([1, 0])).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"], "values must be booleans");
}

#[tokio::test]
async fn malformed_body_is_rejected() {
    // Missing the `values` field is a client error, not a 500.
    let app = router(telemetry::metrics_handle_for_tests());
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "notvalues": [] }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn generates_request_id_when_absent() {
    let app = router(telemetry::metrics_handle_for_tests());
    let res = app
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        res.headers().contains_key("x-request-id"),
        "response must carry a generated x-request-id"
    );
}
