# POOOL Smart Contracts — Deployment Guide

## Overview

This guide covers deploying the `POOOLProperty1155` ERC-1155 contract to Polygon networks.

| Network | Chain ID | Explorer | Faucet |
|---------|----------|----------|--------|
| **Polygon Amoy** (testnet) | 80002 | [amoy.polygonscan.com](https://amoy.polygonscan.com) | [faucet.polygon.technology](https://faucet.polygon.technology/) |
| **Polygon PoS** (mainnet) | 137 | [polygonscan.com](https://polygonscan.com) | N/A |

---

## Prerequisites

### 1. Environment Setup

```bash
# Copy the env template
cd contracts/
cp .env.example .env
```

Edit `.env` with your values:

```env
DEPLOYER_PRIVATE_KEY=0x...    # Private key for deployer wallet
ADMIN_ADDRESS=0x...           # Admin address (receives all roles)
POLYGON_AMOY_RPC_URL=https://rpc-amoy.polygon.technology
POLYGONSCAN_API_KEY=...       # From polygonscan.com/myapikey
```

### 2. Fund Your Deployer Wallet

Get free testnet POL from the [Polygon Faucet](https://faucet.polygon.technology/):
1. Connect your wallet
2. Select "Amoy" network
3. Request 0.5 POL (sufficient for ~50 deployments)

### 3. Get a PolygonScan API Key

1. Create account at [polygonscan.com](https://polygonscan.com)
2. Go to **API Keys** → **Create API Key**
3. Add to your `.env` as `POLYGONSCAN_API_KEY`

---

## Deployment Commands

### Step 1: Deploy to Polygon Amoy Testnet

```bash
cd contracts/

# Load env variables
source .env

# Deploy + verify on PolygonScan
forge script script/Deploy.s.sol:DeployPOOOLProperty1155 \
  --rpc-url polygon_amoy \
  --broadcast \
  --verify \
  --etherscan-api-key $POLYGONSCAN_API_KEY \
  -vvvv
```

Save the **Contract Address** from the output — you'll need it for the next step.

### Step 2: Post-Deployment Setup

```bash
# Set the deployed contract address
export CONTRACT_ADDRESS=0x_YOUR_DEPLOYED_ADDRESS
export SETTLEMENT_WALLET=0x_YOUR_BACKEND_WALLET

forge script script/PostDeploySetup.s.sol:PostDeploySetup \
  --rpc-url polygon_amoy \
  --broadcast \
  -vvvv
```

This will:
- Grant `SETTLEMENT_ROLE` to your backend wallet
- Whitelist the settlement wallet

### Step 3: Verify Deployment

```bash
# Check contract on PolygonScan
echo "View contract: https://amoy.polygonscan.com/address/$CONTRACT_ADDRESS"

# Verify roles are set (read-only call)
cast call $CONTRACT_ADDRESS \
  "hasRole(bytes32,address)(bool)" \
  $(cast keccak "SETTLEMENT_ROLE") \
  $SETTLEMENT_WALLET \
  --rpc-url polygon_amoy
```

---

## Mainnet Deployment

> ⚠️ **Only deploy to mainnet AFTER:**
> 1. Testnet deployment is fully verified
> 2. Smart contract audit is complete
> 3. All integration tests pass against testnet contract
> 4. Admin multisig wallet is set up (e.g., Gnosis Safe)

```bash
forge script script/Deploy.s.sol:DeployPOOOLProperty1155 \
  --rpc-url polygon_mainnet \
  --broadcast \
  --verify \
  --etherscan-api-key $POLYGONSCAN_API_KEY \
  -vvvv
```

---

## Local Testing with Anvil

```bash
# Terminal 1: Start local Polygon fork
anvil --fork-url $POLYGON_AMOY_RPC_URL

# Terminal 2: Deploy to local fork
forge script script/Deploy.s.sol:DeployPOOOLProperty1155 \
  --rpc-url http://localhost:8545 \
  --broadcast \
  -vvvv
```

---

## Contract Interaction (Post-Deploy)

### Create a New Property Asset

```bash
# Create tokenId=1 with 100,000 shares minted to admin
cast send $CONTRACT_ADDRESS \
  "createAsset(uint256,uint256,address,string)" \
  1 100000 $ADMIN_ADDRESS "ipfs://QmPropertyMetadata1/metadata.json" \
  --rpc-url polygon_amoy \
  --private-key $DEPLOYER_PRIVATE_KEY
```

### Whitelist a User

```bash
cast send $CONTRACT_ADDRESS \
  "setWhitelisted(address,bool)" \
  0xUSER_ADDRESS true \
  --rpc-url polygon_amoy \
  --private-key $DEPLOYER_PRIVATE_KEY
```

### Check Balance

```bash
cast call $CONTRACT_ADDRESS \
  "balanceOf(address,uint256)(uint256)" \
  $ADMIN_ADDRESS 1 \
  --rpc-url polygon_amoy
```

### Execute a Settlement Batch

```bash
# This is what the Rust backend calls
cast send $CONTRACT_ADDRESS \
  "settleBatch(address[],address[],uint256[],uint256[])" \
  "[0xSELLER]" "[0xBUYER]" "[1]" "[100]" \
  --rpc-url polygon_amoy \
  --private-key $SETTLEMENT_PRIVATE_KEY
```

---

## Architecture Reference

```
contracts/
├── foundry.toml              # Config + RPC endpoints + PolygonScan verification
├── .env.example              # Environment variable template
├── src/
│   └── POOOLProperty1155.sol # Core ERC-1155 contract
├── script/
│   ├── Deploy.s.sol          # Deployment script
│   └── PostDeploySetup.s.sol # Post-deploy role/whitelist configuration
├── test/
│   ├── POOOLProperty1155.t.sol           # 66 unit tests
│   ├── POOOLProperty1155.fuzz.t.sol      # 12 fuzz tests (10k runs each)
│   └── POOOLProperty1155.invariant.t.sol # 2 invariant tests (25.6k calls)
└── lib/
    ├── forge-std/             # Foundry standard library
    └── openzeppelin-contracts/ # OpenZeppelin v5.1.0
```

---

## Security Checklist (Pre-Mainnet)

- [ ] Smart contract audit completed
- [ ] Admin address is a multisig (Gnosis Safe)
- [ ] Settlement wallet uses hardware wallet or HSM
- [ ] Max ownership limit verified (80%)
- [ ] Emergency pause tested on testnet
- [ ] All test suites passing (80/80)
- [ ] Backend integration tested against testnet contract
- [ ] Private keys stored in secure secret manager (not env vars)
- [ ] Monitoring set up for contract events
- [ ] Incident response plan documented
