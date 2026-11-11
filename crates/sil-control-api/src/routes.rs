use crate::Scope;

/// Une route de l'API. Cette table est la **source unique** du routeur et de l'OpenAPI.
#[derive(Debug, Clone, Copy)]
pub struct Route {
    pub method: &'static str,
    pub path: &'static str,
    /// `None` = publique.
    pub scope: Option<Scope>,
    pub summary: &'static str,
}

pub const ROUTES: &[Route] = &[
    Route {
        method: "post",
        path: "/v1/settlements",
        scope: Some(Scope::Submit),
        summary: "Soumet un règlement signé par les approbateurs (CBOR) ; ce nœud paie les frais",
    },
    Route {
        method: "get",
        path: "/health",
        scope: None,
        summary: "Le nœud répond",
    },
    Route {
        method: "get",
        path: "/v1/openapi.json",
        scope: None,
        summary: "Description de l'API",
    },
    Route {
        method: "put",
        path: "/v1/content",
        scope: Some(Scope::Submit),
        summary: "Envoie un contenu (octets) et retourne son identifiant",
    },
    Route {
        method: "post",
        path: "/v1/events",
        scope: Some(Scope::Submit),
        summary: "Envoie un événement signé (CBOR canonique)",
    },
    Route {
        method: "get",
        path: "/v1/jobs/{id}/outputs",
        scope: Some(Scope::Read),
        summary: "Résultat brut de chaque fragment d'un job final (hexadécimal, petits résultats)",
    },
    Route {
        method: "get",
        path: "/v1/jobs/{id}/input",
        scope: Some(Scope::Read),
        summary: "Contenu d'entrée d'un job (octets bruts, tel que stocké par le nœud)",
    },
    Route {
        method: "get",
        path: "/v1/jobs/{id}/archive",
        scope: Some(Scope::Read),
        summary: "Résultats bruts de tous les fragments d'un job final, concaténés (lots de fichiers)",
    },
    Route {
        method: "get",
        path: "/v1/jobs/{id}/image",
        scope: Some(Scope::Read),
        summary: "Image assemblée d'un job de rendu (octets RGB24, largeur et hauteur en en-têtes)",
    },
    Route {
        method: "get",
        path: "/v1/jobs/{id}",
        scope: Some(Scope::Read),
        summary: "État dérivé d'un job",
    },
    Route {
        method: "get",
        path: "/v1/peers",
        scope: Some(Scope::Read),
        summary: "Pairs connectés",
    },
    Route {
        method: "get",
        path: "/v1/settlements",
        scope: Some(Scope::Read),
        summary: "État des règlements",
    },
    Route {
        method: "get",
        path: "/v1/worker",
        scope: Some(Scope::Read),
        summary: "Activité de ce nœud comme worker : événements, réputation, règlements vus",
    },
];
