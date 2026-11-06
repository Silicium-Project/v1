use super::support::*;
use crate::*;
use axum::http::StatusCode;
use serde_json::json;
use sil_types::ContentId;

#[tokio::test]
async fn content_is_sent_as_bytes_and_a_path_is_never_interpreted() {
    let (app, backend) = app();
    let submitter = secret(SUBMITTER);
    // Un « chemin » envoyé comme corps est du contenu : il n'est jamais lu sur le disque.
    let (status, body) = call(
        &app,
        "PUT",
        "/v1/content",
        Some(submitter),
        b"/etc/passwd".to_vec(),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["content_id"], ContentId::from_bytes([5; 32]).to_hex());
    assert_eq!(body["size"], 11);
    assert_eq!(backend.stored.lock().unwrap()[0], b"/etc/passwd");
    assert!(
        call(
            &app,
            "PUT",
            "/v1/content?path=/etc/passwd",
            Some(submitter),
            vec![]
        )
        .await
        .0
        .is_success()
    );
}

#[tokio::test]
async fn body_sizes_are_bounded() {
    let (app, backend) = app_with(ApiLimits {
        max_upload_bytes: 100,
        max_event_bytes: 10,
        ..ApiLimits::default()
    });
    let token = secret(SUBMITTER);
    assert_eq!(
        call(&app, "PUT", "/v1/content", Some(token), vec![0; 101])
            .await
            .0,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert_eq!(
        call(&app, "PUT", "/v1/content", Some(token), vec![0; 100])
            .await
            .0,
        StatusCode::CREATED
    );
    assert_eq!(
        call(&app, "POST", "/v1/events", Some(token), vec![0; 11])
            .await
            .0,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert!(backend.events.lock().unwrap().is_empty());
}

#[tokio::test]
async fn backend_errors_become_clean_http_errors() {
    let (app, _) = app();
    let token = secret(SUBMITTER);
    let (status, body) = call(&app, "POST", "/v1/events", Some(token), b"bad".to_vec()).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("invalide"));
    let (status, _) = call(&app, "POST", "/v1/events", Some(token), vec![]).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "événement vide");
}

#[tokio::test]
async fn jobs_are_found_or_not_and_ids_are_validated() {
    let (app, _) = app();
    let token = secret(READER);
    let known = format!("/v1/jobs/{}", "01".repeat(32));
    let unknown = format!("/v1/jobs/{}", "02".repeat(32));
    assert_eq!(
        call(&app, "GET", &known, Some(token), vec![]).await,
        (StatusCode::OK, json!({"status": "Active"}))
    );
    assert_eq!(
        call(&app, "GET", &unknown, Some(token), vec![]).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call(&app, "GET", "/v1/jobs/zz", Some(token), vec![])
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
}
