# Source provenance

This Rust demo adapts the MIT-licensed bqAtlas Angular application at source revision `f1d0d3ee1e7cbf044c5b2bf0e21ae89859b07757` from [binaryquest/bqAtlas](https://github.com/binaryquest/bqAtlas). Its imported application views live under `samples/crm/web/src/`, including `crm/`, `sales/`, `showcase/` and `control-docs/`. Rust-specific startup, proxy and presentation changes are maintained here.

The Atlas controls and Angular framework services remain shared packages maintained in bqAtlas. The local archives for `@bqatlas/contracts`, `@bqatlas/ui` and `@bqatlas/angular` are recorded with source revision and SHA256 checksums in [vendor/manifest.json](vendor/manifest.json). Verify them with `node dev/verify-frontend.mjs` from the repository root. There is no runtime dependency on the original checkout.

The upstream Atlas UI originated in the owner's Atlas workspace on 25 September 2026. Showcase examples were adapted into bqAtlas on 26 September 2026. bqStart supplied architecture concepts; its application and private credentials were not copied. These reused sources are released under the [root MIT license](../../../LICENSE) as directed by the owner. Third-party dependencies retain their own licenses; see [the dependency inventory](../../../docs/DEPENDENCY-LICENSES.json). Public registry publication remains deferred.
