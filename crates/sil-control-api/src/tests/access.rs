use super::support::*;
use crate::*;
use axum::http::StatusCode;

#[tokio::test]
async fn public_routes_need_no_token_and_protected_ones_refuse_anonymous_callers() {
    let (app, _) = app();
    assert_eq!(
        call(&app, "GET", "/health", None, vec![]).await.0,
        StatusCode::OK
    );
    assert_eq!(
        call(&app, "GET", "/v1/openapi.json", None, vec![]).await.0,
        StatusCode::OK
    );
    for (method, path) in [
        ("GET", "/v1/peers"),
        ("GET", "/v1/settlements"),
        (
            "GET",
            "/v1/jobs/0101010101010101010101010101010101010101010101010101010101010101",
        ),
        ("PUT", "/v1/content"),
        ("POST", "/v1/events"),
    ] {
        let (status, _) = call(&app, method, path, None, b"x".to_vec()).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {path}");
    }
}

#[tokio::test]
async fn wrong_tokens_and_look_alikes_are_refused() {
    let (app, _) = app();
    for token in [
        "",
        "nope",
        &secret(READER)[..20],
        &format!("{}x", secret(READER)),
    ] {
        let (status, _) = call(&app, "GET", "/v1/peers", Some(token), vec![]).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{token}");
    }
    let (status, _) = call(&app, "GET", "/v1/peers", Some(secret(READER)), vec![]).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn scopes_separate_reading_from_submitting() {
    let (app, _) = app();
    let reader = secret(READER);
    let submitter = secret(SUBMITTER);
    assert_eq!(
        call(&app, "PUT", "/v1/content", Some(reader), b"x".to_vec())
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(&app, "POST", "/v1/events", Some(reader), b"x".to_vec())
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(&app, "GET", "/v1/peers", Some(submitter), vec![])
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(&app, "PUT", "/v1/content", Some(submitter), b"x".to_vec())
            .await
            .0,
        StatusCode::CREATED
    );
    let admin = secret(ADMIN);
    assert_eq!(
        call(&app, "GET", "/v1/peers", Some(admin), vec![]).await.0,
        StatusCode::OK
    );
    assert_eq!(
        call(&app, "POST", "/v1/events", Some(admin), b"ok".to_vec())
            .await
            .0,
        StatusCode::ACCEPTED
    );
}

#[tokio::test]
async fn a_token_that_floods_is_rate_limited_without_affecting_others() {
    let (app, _) = app_with(ApiLimits {
        burst: 3,
        per_sec: 0,
        ..ApiLimits::default()
    });
    let (reader, admin) = (secret(READER), secret(ADMIN));
    for _ in 0..3 {
        assert_eq!(
            call(&app, "GET", "/v1/peers", Some(reader), vec![]).await.0,
            StatusCode::OK
        );
    }
    assert_eq!(
        call(&app, "GET", "/v1/peers", Some(reader), vec![]).await.0,
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(
        call(&app, "GET", "/v1/peers", Some(admin), vec![]).await.0,
        StatusCode::OK
    );
}

#[tokio::test]
async fn repeated_bad_tokens_are_throttled() {
    let (app, _) = app();
    let mut throttled = false;
    for _ in 0..40 {
        let (status, _) = call(&app, "GET", "/v1/peers", Some("wrong"), vec![]).await;
        throttled |= status == StatusCode::TOO_MANY_REQUESTS;
    }
    assert!(throttled, "l'essai de jetons à répétition doit être freiné");
}

#[tokio::test]
async fn every_documented_route_exists_and_every_protected_one_is_guarded() {
    let (app, _) = app();
    let spec = call(&app, "GET", "/v1/openapi.json", None, vec![]).await.1;
    for route in ROUTES {
        assert!(
            spec["paths"][route.path][route.method].is_object(),
            "{}",
            route.path
        );
        let path = route.path.replace("{id}", &"01".repeat(32));
        let (status, _) = call(&app, &route.method.to_uppercase(), &path, None, vec![]).await;
        assert_ne!(
            status,
            StatusCode::NOT_FOUND,
            "{} absente du routeur",
            route.path
        );
        if route.scope.is_some() {
            assert_eq!(
                status,
                StatusCode::UNAUTHORIZED,
                "{} non protégée",
                route.path
            );
        }
    }
}
