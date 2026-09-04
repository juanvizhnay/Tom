# Tom

<p align="center">
  <img src="apps/tom-desktop/assets/orb-states/idle.png" width="180" alt="Tom's idle orb" />
</p>

<p align="center">
  <strong>A native, local-first desktop assistant that adapts to the way you work.</strong>
</p>

Tom is an experimental desktop assistant built to make everyday work calmer and more intentional. It combines controllable memory, explainable routine suggestions, flexible AI-provider settings, and a small orb that can stay close without taking over your screen.

Choose **Tom** for a professional tone or **Tomy** for a friendlier, more relaxed personality. Both personalities have the same capabilities and permissions.

> [!IMPORTANT]
> Tom is in early development. The desktop experience, local persistence, secure credential storage, and initial routine flow are working. Model inference, real background routine learning, and operating-system integrations are still on the roadmap.

## What makes Tom different

- **Native desktop experience** — built with Rust and Slint, without a browser-based runtime.
- **Local-first by design** — profile data, notes, and non-secret settings stay in a local SQLite database.
- **User-controlled memory** — Tom is designed around memory that can be inspected, edited, and forgotten.
- **Explainable assistance** — suggestions separate what Tom observed, inferred, and proposes to do.
- **Adaptable profiles** — onboarding adjusts Tom for developers, office professionals, students, creators, or general use.
- **Bring your own AI** — configure a cloud provider, an OpenAI-compatible endpoint, or a local model endpoint.
- **Secure credentials** — API keys are stored in Windows Credential Manager, never in the SQLite database.
- **Orb mode** — collapse the dashboard into a movable, resizable companion with visual states.

## Current experience

On the first launch, Tom guides you through a five-step setup:

1. Choose between Tom and Tomy.
2. Select your profession or primary type of work.
3. Set how proactive the assistant should be.
4. Configure an optional AI provider.
5. Review the privacy model before entering the dashboard.

The main application currently includes:

- **Today** — quick note capture and an activity pulse with explainable suggestions.
- **Memory** — a local view of what Tom remembers.
- **Routines** — suggestions that can be accepted or dismissed explicitly.
- **Settings** — personality, profession, initiative level, provider metadata, and secure API-key management.
- **Orb mode** — a compact window that can be dragged anywhere, resized from 96 to 288 pixels, and restored to the full dashboard.

## Technology

| Area | Technology |
| --- | --- |
| Language | Rust 2024, Rust 1.92+ |
| Desktop UI | Slint 1.17 |
| Local storage | SQLite through `rusqlite` |
| Secret storage | Windows Credential Manager through `keyring` |
| Project structure | Cargo workspace |
| License | MIT |

## Architecture

```text
Tom/
├── apps/
│   └── tom-desktop/          # Slint UI, desktop controller, credentials, orb assets
├── crates/
│   └── tom-core/             # Domain models, persistence, and application logic
├── AGENTS.md                 # Product and engineering context for coding agents
├── SECURITY.md               # Security and secret-handling policy
└── README.md
```

The UI delegates state and actions to the desktop controller. Reusable domain and persistence logic lives in `tom-core`, keeping the interface separate from the assistant's core behavior.

## Run locally

### Prerequisites

- Windows 11
- Rust 1.92 or newer with the MSVC toolchain
- Visual Studio Build Tools with the Windows SDK

From the repository root:

```powershell
cargo run -p tom-desktop
```

The first launch opens the onboarding flow. Runtime data is created in the operating system's local application-data directory rather than inside the repository.

## AI configuration

Tom's settings support provider metadata, custom endpoints, model names, and local model paths. API keys are stored separately in the operating system's credential vault.

The configuration layer is implemented, but Tom does **not** send inference requests yet. Network clients and local-model execution will be added as explicit, optional capabilities so users remain in control of when data leaves their device.

## Privacy and security

Tom follows a simple rule: private context should never be observed, stored, or shared without clear user permission.

- Local data is stored in SQLite.
- Secrets are never written to SQLite or committed to the repository.
- Cloud AI is optional.
- Suggestions are intended to explain their evidence and reasoning.
- Future integrations must request permission before observing or acting.

Please read [SECURITY.md](SECURITY.md) before reporting a vulnerability or handling credentials during development.

## Development

Run the complete validation suite with:

```powershell
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The repository denies unsafe Rust and treats Clippy's `all` and `pedantic` lint groups as warnings during normal development.

## Roadmap

- [x] Native desktop shell and onboarding
- [x] Tom and Tomy personalities
- [x] Profession and assistance-level profiles
- [x] Local notes and persisted settings
- [x] Secure API-key storage on Windows
- [x] Explainable routine-suggestion flow
- [x] Movable and resizable orb mode
- [ ] Cloud and OpenAI-compatible inference clients
- [ ] Local-model execution
- [ ] Searchable, editable, and forgettable memory controls
- [ ] Permissioned routine detection and background assistance
- [ ] Optional productivity-tool and operating-system integrations
- [ ] Cross-platform credential backends and packaging

## Contributing

Tom is being shaped in public and contributions will be welcome as the project matures. Before making changes, read [AGENTS.md](AGENTS.md) for the product principles, current architecture, and implementation constraints.

Please keep new capabilities local-first, permission-based, explainable, and honest about what is already implemented.

## License

Tom is available under the [MIT License](LICENSE).
