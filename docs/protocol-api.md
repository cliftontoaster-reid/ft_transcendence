# Protocol API

Cross-service contracts are defined as Protocol Buffers in `proto/`. The
packages are versioned under `dionysus/<domain>/v1`.

## Packages

| Package | Scope |
| --- | --- |
| `common/v1` | Shared assets and image types |
| `encoder/v1` | FFmpeg job service |
| `media/v1` | Media, people, and credits |
| `playback/v1` | Playback-oriented messages |
| `room/v1` | Shared rooms |
| `user/v1` | User data |
| `chat/v1` | Chat contracts (currently reserved) |
| `search/v1` | Search contracts (currently reserved) |

## Compatibility rules

- Never reuse a field number for a different meaning.
- Reserve removed field numbers and names.
- Add fields instead of changing the meaning of existing fields.
- Use explicit package and message names.
- Keep generated clients out of hand-edited source changes.

The Rust services compile protobufs with tonic/prost. The frontend Docker build
uses Buf to generate TypeScript clients. A contract change therefore needs
validation in both ecosystems.

## Change checklist

1. Edit the `.proto` source.
2. Run the relevant Rust and frontend generation/build commands.
3. Review generated API changes.
4. Update the service implementation and tests.
5. Document behavior changes and rollout assumptions.
