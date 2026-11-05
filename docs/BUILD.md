# Contributor builds

## JavaScript service, contracts and web

Install Node 22.16 or later within Node 22 and npm 10 or 11. From the repository root:

```sh
npm ci
npm run contracts:check
npm run check:server
npm run check:web
npm run test:source
npm run build:server
npm run build:web
```

These commands validate generated contracts, TypeScript and browser source and compile/copy service/web output into ignored `dist`. They do not start a service, database, desktop app or game, and do not prove actual Noita/multiplayer behavior. Browser tests use explicitly synthetic fixtures rather than a captured service response. The historical private detector-review receipt is not included or claimed as fresh evidence.

The source-only npm commands intentionally replace the original private fixture/publication command collection. Additional focused tests are retained under `apps/service/test` and `packages/contracts/test`; integration tests need an explicitly owned disposable PostgreSQL instance, fresh namespace, matching local fixture/native inputs and cleanup. Do not run them against production. No test database or original private approvals are bundled.

For the old in-memory development prototype only, `npm run dev:mock` listens on loopback with `NT2_G0_MOCK=1`. It is not the current production service. The current AUTH1 service launcher is `apps/service/scripts/auth1-host.ts`; it requires locally supplied secrets, HTTPS configuration, database target and independently reviewed matching package approvals. `config/auth1.example.json` contains placeholders, not a working deployment. Validation remains fail-closed when approvals are absent. No production startup/deployment claim is made for this export.

## Windows desktop and native components

See [CLIENT_BUILD.md](CLIENT_BUILD.md) for the audited modern Cargo entrypoint and native-source preparation. These instructions are supplied with the Client source audit. A contributor build must not fabricate release approval or bypass exact service compatibility checks. The paid game supplies its own original executable/Lua runtime and resources; do not copy them into the repository.

## Retained historical tools

Some service/native fixture and package-validation implementations refer to historical approval profiles, private reports or scratch build receipts. Those inputs are intentionally absent. They are not public build prerequisites for the source checks above and cannot reproduce an old approved deployment by themselves. Supply newly reviewed corresponding inputs in a private development environment when exercising those paths. Never replace missing approval checks with unconditional success.

Dependency licence files are obtained with the locked npm/Cargo dependencies. Preserve notices when distributing binaries or vendored dependencies, including MPL covered-source availability. The root notices distinguish the Microsoft embedded WebView2 SDK loader from the Rust bindings and separately installed runtime.
