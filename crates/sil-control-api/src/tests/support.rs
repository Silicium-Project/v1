use crate::*;
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use sil_types::{ContentId, JobId};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

pub const READER: &str = "read:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
pub const SUBMITTER: &str = "submit:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
pub const ADMIN: &str = "read,submit:cccccccccccccccccccccccccccccccccccccccccccc";

#[derive(Default)]
pub struct Fake {
    pub stored: Mutex<Vec<Vec<u8>>>,
    pub events: Mutex<Vec<Vec<u8>>>,
}

impl Backend for Fake {
    fn put_content(&self, bytes: Vec<u8>) -> Result<ContentId, ApiError> {
        self.stored.lock().unwrap().push(bytes);
        Ok(ContentId::from_bytes([5; 32]))
    }

    fn submit_event(&self, bytes: Vec<u8>) -> Result<EventReceipt, ApiError> {
        if bytes == b"bad" {
            return Err(ApiError::BadRequest("événement invalide".into()));
        }
        self.events.lock().unwrap().push(bytes);
        Ok(EventReceipt {
            accepted: 1,
            pending: false,
        })
    }

    fn job(&self, job: &JobId) -> Result<Option<Value>, ApiError> {
        Ok((job.as_bytes()[0] == 1).then(|| json!({ "status": "Active" })))
    }

    fn job_outputs(&self, job: &JobId) -> Result<Vec<FragmentOutput>, ApiError> {
        match job.as_bytes()[0] {
            1 => Ok(vec![
                FragmentOutput {
                    fragment: 0,
                    bytes: vec![0xab, 0xcd],
                },
                FragmentOutput {
                    fragment: 1,
                    bytes: vec![0x01],
                },
            ]),
            _ => Err(ApiError::NotFound),
        }
    }

    fn job_image(&self, job: &JobId) -> Result<JobImage, ApiError> {
        match job.as_bytes()[0] {
            1 => Ok(JobImage {
                width: 2,
                height: 1,
                rgb: vec![1, 2, 3, 4, 5, 6],
            }),
            2 => Err(ApiError::BadRequest(
                "ce workload ne produit pas d'image".into(),
            )),
            _ => Err(ApiError::NotFound),
        }
    }

    fn submit_settlement(&self, bytes: Vec<u8>) -> Result<Value, ApiError> {
        if bytes.is_empty() {
            return Err(ApiError::BadRequest("règlement vide".into()));
        }
        Ok(serde_json::json!({ "tx": "fake-tx" }))
    }

    fn peers(&self) -> Result<Value, ApiError> {
        Ok(json!([]))
    }

    fn settlements(&self) -> Result<Value, ApiError> {
        Ok(json!([]))
    }

    fn worker(&self) -> Result<Value, ApiError> {
        Ok(json!({ "worker": "00", "jobs_touched": 0 }))
    }
}

pub fn app_with(limits: ApiLimits) -> (Router, Arc<Fake>) {
    let tokens =
        TokenSet::parse(&format!("{READER}\n# commentaire\n{SUBMITTER}\n{ADMIN}\n")).unwrap();
    let backend = Arc::new(Fake::default());
    (router(backend.clone(), tokens, limits), backend)
}

pub fn app() -> (Router, Arc<Fake>) {
    app_with(ApiLimits::default())
}

pub async fn call(
    app: &Router,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Vec<u8>,
) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(path);
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::from(body)).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

pub fn secret(line: &str) -> &str {
    line.split_once(':').unwrap().1
}
