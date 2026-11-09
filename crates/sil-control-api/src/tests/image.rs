use super::support::*;
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{HeaderMap, Request, StatusCode},
};
use sil_types::JobId;
use tower::ServiceExt;

async fn image(app: &Router, job: JobId, token: Option<&str>) -> (StatusCode, HeaderMap, Vec<u8>) {
    let mut request = Request::builder()
        .method("GET")
        .uri(format!("/v1/jobs/{}/image", job.to_hex()));
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, headers, bytes.to_vec())
}

#[tokio::test]
async fn a_finished_image_comes_as_raw_pixels_with_its_size_in_headers() {
    let (app, _) = app();
    let (status, headers, bytes) =
        image(&app, JobId::from_bytes([1; 32]), Some(secret(READER))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers["content-type"], "application/octet-stream");
    assert_eq!(headers["x-image-width"], "2");
    assert_eq!(headers["x-image-height"], "1");
    assert_eq!(bytes, vec![1, 2, 3, 4, 5, 6]);
}

#[tokio::test]
async fn a_workload_without_image_and_an_unknown_job_are_refused() {
    let (app, _) = app();
    let token = secret(READER);
    assert_eq!(
        image(&app, JobId::from_bytes([2; 32]), Some(token)).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        image(&app, JobId::from_bytes([3; 32]), Some(token)).await.0,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn the_image_needs_a_token() {
    let (app, _) = app();
    assert_eq!(
        image(&app, JobId::from_bytes([1; 32]), None).await.0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn the_outputs_of_each_fragment_come_in_hexadecimal_in_order() {
    let (app, _) = app();
    let request = Request::builder()
        .method("GET")
        .uri(format!(
            "/v1/jobs/{}/outputs",
            JobId::from_bytes([1; 32]).to_hex()
        ))
        .header("authorization", format!("Bearer {}", secret(READER)))
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(body["fragments"][0]["fragment"], 0);
    assert_eq!(body["fragments"][0]["hex"], "abcd");
    assert_eq!(body["fragments"][1]["hex"], "01");
}
