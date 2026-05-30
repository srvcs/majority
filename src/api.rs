use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use utoipa::{OpenApi, ToSchema};

/// This service's identity. `srvcs-majority` is a leaf: it depends on no other
/// service. The majority predicate is computed entirely from the local list of
/// booleans, so there is nothing to fan out to.
pub const SERVICE: &str = "srvcs-majority";
pub const CONCERN: &str = "logic: are more than half of the values true";
pub const DEPENDS_ON: &[&str] = &[];

#[derive(Serialize, ToSchema)]
pub struct Info {
    pub service: &'static str,
    pub concern: &'static str,
    pub depends_on: Vec<&'static str>,
}

/// `GET /` — service identity (srvcs service standard).
#[utoipa::path(get, path = "/", responses((status = 200, body = Info)))]
pub async fn index() -> Json<Info> {
    Json(Info {
        service: SERVICE,
        concern: CONCERN,
        depends_on: DEPENDS_ON.to_vec(),
    })
}

#[derive(Deserialize, ToSchema)]
pub struct EvalRequest {
    /// The list of booleans to test. Each element must be a JSON boolean. An
    /// empty list yields `false` (no majority).
    #[schema(value_type = Object)]
    pub values: Vec<Value>,
}

#[derive(Serialize, ToSchema)]
pub struct MajorityResponse {
    #[schema(value_type = Object)]
    pub values: Vec<Value>,
    pub result: bool,
}

/// The single concern: are strictly more than half of the values `true`?
///
/// Returns `None` if any element is not a JSON boolean. For a valid list the
/// result is `(count_true * 2) > len`, which is `false` for the empty list and
/// `false` on a tie.
pub fn majority(values: &[Value]) -> Option<bool> {
    let mut count = 0usize;
    for v in values {
        match v.as_bool() {
            Some(true) => count += 1,
            Some(false) => {}
            None => return None,
        }
    }
    Some(count * 2 > values.len())
}

/// `POST /` — are more than half of the values in the list `true`?
///
/// All work is local: every element is read with `Value::as_bool`. A
/// non-boolean element is a client error (`422`). Otherwise the result is
/// `(true_count * 2) > values.len()`, so the empty list and ties are `false`.
#[utoipa::path(
    post,
    path = "/",
    request_body = EvalRequest,
    responses(
        (status = 200, body = MajorityResponse),
        (status = 422, description = "an element of values is not a boolean")
    )
)]
pub async fn evaluate(Json(req): Json<EvalRequest>) -> Response {
    match majority(&req.values) {
        Some(result) => (
            StatusCode::OK,
            Json(json!({ "values": req.values, "result": result })),
        )
            .into_response(),
        None => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({ "error": "values must be booleans" })),
        )
            .into_response(),
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(index, evaluate),
    components(schemas(Info, EvalRequest, MajorityResponse))
)]
pub struct ApiDoc;

/// Serve OpenAPI document
pub async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openapi_documents_routes() {
        let doc = ApiDoc::openapi();
        let root = doc.paths.paths.get("/").expect("path / present");
        assert!(root.get.is_some(), "GET / documented");
        assert!(root.post.is_some(), "POST / documented");
    }

    #[test]
    fn index_reports_identity() {
        // The leaf identity must report no dependencies.
        assert_eq!(SERVICE, "srvcs-majority");
        assert_eq!(CONCERN, "logic: are more than half of the values true");
        assert!(DEPENDS_ON.is_empty());
    }

    #[test]
    fn empty_list_has_no_majority() {
        assert_eq!(majority(&[]), Some(false));
    }

    #[test]
    fn strict_majority_is_true() {
        assert_eq!(
            majority(&[json!(true), json!(true), json!(false)]),
            Some(true)
        );
        assert_eq!(majority(&[json!(true)]), Some(true));
    }

    #[test]
    fn tie_is_false() {
        // A tie is not a strict majority.
        assert_eq!(majority(&[json!(true), json!(false)]), Some(false));
        assert_eq!(
            majority(&[json!(true), json!(true), json!(false), json!(false)]),
            Some(false)
        );
    }

    #[test]
    fn minority_is_false() {
        assert_eq!(
            majority(&[json!(true), json!(false), json!(false)]),
            Some(false)
        );
        assert_eq!(majority(&[json!(false), json!(false)]), Some(false));
    }

    #[test]
    fn non_boolean_element_is_rejected() {
        for bad in [
            json!("true"),
            json!(1),
            json!(0),
            json!(null),
            json!([true]),
            json!({ "v": true }),
        ] {
            assert_eq!(
                majority(&[json!(true), bad.clone()]),
                None,
                "{bad} should be rejected"
            );
        }
    }

    #[tokio::test]
    async fn index_route_reports_identity() {
        let Json(info) = index().await;
        assert_eq!(info.service, "srvcs-majority");
        assert!(info.depends_on.is_empty());
    }
}
