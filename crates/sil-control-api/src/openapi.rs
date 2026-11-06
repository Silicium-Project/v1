use crate::{ROUTES, Scope};
use serde_json::{Map, Value, json};

fn scope_name(scope: Scope) -> &'static str {
    match scope {
        Scope::Read => "read",
        Scope::Submit => "submit",
    }
}

/// Document OpenAPI 3 généré de [`ROUTES`] : il ne peut pas diverger du routeur.
pub(crate) fn document() -> Value {
    let mut paths: Map<String, Value> = Map::new();
    for route in ROUTES {
        let mut operation = json!({
            "summary": route.summary,
            "responses": { "200": { "description": "Succès" }, "4XX": { "description": "Erreur" } },
        });
        if let Some(scope) = route.scope {
            operation["security"] = json!([{ "bearer": [scope_name(scope)] }]);
        }
        let entry = paths
            .entry(route.path.to_string())
            .or_insert_with(|| json!({}));
        entry[route.method] = operation;
    }
    json!({
        "openapi": "3.0.3",
        "info": { "title": "API de contrôle d'un nœud Silicium", "version": "1" },
        "paths": paths,
        "components": { "securitySchemes": { "bearer": { "type": "http", "scheme": "bearer" } } },
    })
}
