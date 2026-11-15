# ADR — Independent repositories (v1)

The v1 monorepo is replaced by a public meta-repository and components pinned by SHA.

- The meta-repository holds the architecture, the ADRs, `components.lock.json` and `.gitmodules`.
- Each component has its own repository, its own history and its own CI. The network is public; the other components
  are private.
- A component update: a pull request in the component, then a pull request in the meta-repository that updates the SHA
  (gitlink and `components.lock.json` together).
- No secret, deployment key, signing certificate or `node.seed` belongs in any repository.
