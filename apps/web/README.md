# apps/web

Interface Silicium : connexion, calculs, machines, compte, pages légales. Vue 3, TypeScript,
Vue Router, sans dépendance de rendu tierce, aucune requête vers un autre domaine.

```bash
npm ci
npm run dev          # http://127.0.0.1:5173, API proxifiée vers 127.0.0.1:8080
npm test             # tests unitaires, parité fr/en, contraste, axe-core
npm run build        # build serveur : dist/ (servi par l'API via SIL_API_WEB_DIR)
npm run build:desktop  # application de bureau (nœud) : dist-desktop/ (VITE_API_BASE)
npm run dev:desktop    # même application, en développement (127.0.0.1:5173)
```

Deux applications distinctes, deux builds :

- **Site** (`src/`, `npm run build`) : soumettre, calculs, machines, compte, pages légales.
- **Application de bureau** (`src/desktop/`, `npm run build:desktop`, config `vite.desktop.config.ts`) :
  tableau de bord du nœud et liaison de la machine. Elle ne contient aucune page du site ; Tauri
  charge son `dist-desktop/`.

Structure (un dossier = une responsabilité) :

| Dossier | Contenu |
|---|---|
| `src/pages` | une page par route du site |
| `src/desktop` | l'application de bureau : shell, routes, pages, composants du nœud |
| `src/components` | composants réutilisables (champ, badge d'état, notice, en-tête) |
| `src/api` | client HTTP, points d'accès, codes d'erreur |
| `src/i18n` | catalogues fr (référence) et en, textes longs légaux |
| `src/platform` | pont vers le shell de bureau (absent dans un navigateur) |
| `src/router` | routes, garde de session, redirections sûres |
| `src/state` | session (sessionStorage) |
| `src/styles` | jetons de couleur (`palette.ts` = source, `tokens.css` = copie testée) |

Documentation : `../../docs/technical/web-and-desktop.md`.
