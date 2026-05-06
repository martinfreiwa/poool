# POOOL Smart Contracts — Deployment Guide

## Overview

POOOL deploys three contracts per chain:

| Contract            | Purpose                                                                         |
|---------------------|---------------------------------------------------------------------------------|
| `POOOLAssetToken`   | ERC-1155 implementation. Cloned via EIP-1167 once per property.                 |
| `IdentityRegistry`  | Single KYC whitelist. Every clone delegates `checkWhitelisted()` to this.       |
| `AssetFactory`      | Spawns the EIP-1167 clones, holds `DEPLOYER_ROLE`.                              |

All asset clones are minimal proxies (≈45 bytes of bytecode each) that
delegate every call to the implementation, so verifying the
implementation on Polygonscan once auto-verifies every clone.

| Network                | Chain ID | Explorer                                       | Faucet                                                   |
|------------------------|----------|-----------------------------------------------|----------------------------------------------------------|
| Polygon Amoy (testnet) | 80002    | [amoy.polygonscan.com](https://amoy.polygonscan.com) | [faucet.polygon.technology](https://faucet.polygon.technology/) |
| Polygon PoS (mainnet)  | 137      | [polygonscan.com](https://polygonscan.com)    | n/a                                                      |

---

## Prerequisites

```bash
# Foundry
curl -L https://foundry.paradigm.xyz | bash && foundryup

# Repo deps
cd contracts/
forge install
```

Environment (`contracts/.env`):

```env
DEPLOYER_PRIVATE_KEY=0x...      # signs the deploy txs
ADMIN_ADDRESS=0x...             # receives DEFAULT_ADMIN_ROLE + DEPLOYER_ROLE +
                                # KYC_ADMIN_ROLE; MUST equal the address derived
                                # from the backend's CHAIN_SETTLEMENT_PRIVATE_KEY
                                # secret. Otherwise mintTo and settleBatch will
                                # hit a wallet whose key the worker doesn't hold.
POLYGON_AMOY_RPC_URL=https://rpc-amoy.polygon.technology
POLYGON_MAINNET_RPC_URL=https://polygon-rpc.com   # or Alchemy/Infura
POLYGONSCAN_API_KEY=...         # from polygonscan.com/myapikey — verifies ALL clones
```

---

## Tests

```bash
forge fmt --check
forge build
forge test -vvv
```

Test files live under `contracts/test/`:

| File                                  | Coverage                              |
|---------------------------------------|---------------------------------------|
| `POOOLAssetFactory.t.sol`             | Factory unit tests                    |
| `POOOLAssetFactory.fuzz.t.sol`        | Factory property fuzz tests           |
| `POOOLAssetToken.fuzz.t.sol`          | Implementation property fuzz tests    |

Fuzz config (`foundry.toml`): 10,000 runs per property.

---

## Deploy

The deploy script is **chain-agnostic and always fresh** — it never
reuses an existing implementation address, so it can't accidentally
spawn clones on top of a non-existent implementation (which is what
stranded the original Demo Villa supply on a wallet whose key was
lost).

### Testnet (Polygon Amoy)

```bash
forge script script/Deploy.s.sol:DeployPOOOL \
  --rpc-url $POLYGON_AMOY_RPC_URL \
  --broadcast --verify
```

### Mainnet (Polygon PoS)

```bash
forge script script/Deploy.s.sol:DeployPOOOL \
  --rpc-url $POLYGON_MAINNET_RPC_URL \
  --broadcast --verify
```

The deploy emits three contract addresses. Copy them into the backend
Cloud Run env:

```bash
gcloud run services update poool-backend --region <region> \
  --update-env-vars="\
CHAIN_IMPLEMENTATION_ADDRESS=<impl>,\
CHAIN_IDENTITY_REGISTRY_ADDRESS=<registry>,\
CHAIN_CONTRACT_ADDRESS=<factory>,\
CHAIN_SETTLEMENT_ADDRESS=$ADMIN_ADDRESS,\
CHAIN_NETWORK=polygon,CHAIN_ID=137"
```

The backend's `resolve_settlement_address()` derives the address from
the signing key first, so as long as `CHAIN_SETTLEMENT_PRIVATE_KEY`
matches `ADMIN_ADDRESS` here, the env var above is informational only.

---

## Per-property tokenization

Asset clones are deployed by the backend at admin tokenization time, NOT
by this script. Each call to `AssetFactory.deployAsset()` spawns a fresh
EIP-1167 clone and mints `initialSupply` to the platform treasury
(= the SETTLEMENT_PRIVATE_KEY's derived address).

Trigger via the admin UI: `/admin/asset-tokenize?id=<asset_uuid>`.

---

## Verification

`--verify` only works if `POLYGONSCAN_API_KEY` is set in your env.
If you skipped it during deploy, run after the fact:

```bash
forge verify-contract <implementation_address> \
  src/POOOLAssetToken.sol:POOOLAssetToken \
  --chain 137 \
  --etherscan-api-key $POLYGONSCAN_API_KEY
```

Verifying the implementation auto-applies to every EIP-1167 clone.

---

## Operational notes

- **`settleBatch` bypasses ERC1155Receiver acceptance.** Only EOAs (or
  contracts known to implement `onERC1155Received`) should ever be on
  the IdentityRegistry whitelist. The KYC pipeline gates this.
- **Mint cap is 80 % of supply per holder** (`MAX_OWNERSHIP_BPS`).
  Primary issuance to a single wallet beyond that limit will revert
  with `MaxOwnershipExceeded`. Backend's primary-settlement worker
  caps each transfer at `LEAST(order.qty, investments.tokens_owned)`
  to dodge this.
- **`uri(tokenId)`** returns `""` for any id ≠ 1, per ERC-1155 metadata
  spec. The single asset id is always `ASSET_TOKEN_ID = 1`.
