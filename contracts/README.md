# POOOL Smart Contracts

ERC-1155 tokenized real estate on Polygon. Each property = one
EIP-1167 clone of `POOOLAssetToken`, all sharing a single
`IdentityRegistry` for KYC enforcement.

```
                     ┌──────────────────────┐
                     │  AssetFactory        │  spawns clones
                     │  (DEPLOYER_ROLE)     │
                     └──────────┬───────────┘
                                │ EIP-1167
                                ▼
   ┌──────────────┐    ┌──────────────────┐    ┌──────────────┐
   │ Demo Villa   │    │  Villa Pillada   │    │  Apartment   │  …
   │ (ERC-1155)   │    │  (ERC-1155)      │    │  (ERC-1155)  │
   └──────┬───────┘    └────────┬─────────┘    └──────┬───────┘
          │                     │                     │
          ▼                     ▼                     ▼
                     ┌──────────────────────┐
                     │  IdentityRegistry    │  KYC whitelist
                     │  (KYC_ADMIN_ROLE)    │
                     └──────────────────────┘
```

## Layout

| Path                                     | What                                    |
|------------------------------------------|-----------------------------------------|
| `src/POOOLAssetToken.sol`                | ERC-1155 implementation (cloneable).    |
| `src/IdentityRegistry.sol`               | Single shared KYC whitelist.            |
| `src/AssetFactory.sol`                   | Deploys clones, holds DEPLOYER_ROLE.    |
| `script/Deploy.s.sol`                    | Chain-agnostic full-stack deploy.       |
| `script/CreateImplementation.s.sol`      | Standalone implementation deploy (rare).|
| `test/POOOLAssetFactory.{t,fuzz.t}.sol`  | Factory tests.                          |
| `test/POOOLAssetToken.fuzz.t.sol`        | Token property fuzz tests.              |

## Quick start

```bash
forge install
forge build
forge test -vvv
```

## Deploy

See [`DEPLOYMENT.md`](./DEPLOYMENT.md). One-liner for testnet:

```bash
forge script script/Deploy.s.sol:DeployPOOOL \
  --rpc-url $POLYGON_AMOY_RPC_URL --broadcast --verify
```

## Key invariants

- **Whitelist gate** — `_update` reverts on transfer to a
  non-whitelisted address. Mint and burn bypass the gate (mintTo is
  whitelisted explicitly at `initialize()`; burning to address(0) is
  always allowed).
- **80 % ownership cap** — single holders cannot exceed 80 % of supply
  (`MAX_OWNERSHIP_BPS = 8000`). Mints from address(0) skip the check
  so initial supply can land in treasury.
- **`settleBatch` is operator-only** — only `SETTLEMENT_ROLE` can move
  tokens between two arbitrary addresses without sender approval. This
  bypasses the ERC1155Receiver acceptance hook by design (see contract
  doc note); operator must guarantee recipients are EOAs or compatible
  contracts.
- **`nonReentrant`** on `mint` and `settleBatch` for defense-in-depth.

## Audit checklist

- Foundry fuzz suite: 10k runs per property.
- Slither / Mythril: not yet run; see TODO before mainnet.
- External audit: pending pre-launch.

## Build / contribute

Foundry docs: https://book.getfoundry.sh/

```bash
forge fmt          # autoformat
forge fmt --check  # CI-style check
forge build
forge test
forge coverage     # line coverage report
```
