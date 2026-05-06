// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Script} from "forge-std/Script.sol";
import {AssetFactory} from "../src/AssetFactory.sol";
import {IdentityRegistry} from "../src/IdentityRegistry.sol";
import {POOOLAssetToken} from "../src/POOOLAssetToken.sol";

/// @title DeployPOOOL — chain-agnostic deploy script.
/// @notice Deploys the full POOOL infrastructure (Implementation +
///         IdentityRegistry + AssetFactory) on whichever chain is
///         passed via `--rpc-url`. **Always fresh** — no hardcoded
///         contract addresses, so the script can't accidentally
///         spawn clones on top of a non-existent implementation
///         (which is what stranded the original Demo Villa supply).
///
/// Required env:
///   DEPLOYER_PRIVATE_KEY  — key that signs the deploy txs.
///   ADMIN_ADDRESS         — receives DEFAULT_ADMIN_ROLE + DEPLOYER_ROLE
///                           on the registry and the factory. Must be
///                           the same address you use as
///                           CHAIN_SETTLEMENT_ADDRESS in the backend
///                           env, otherwise mintTo / settlement will
///                           hit a wallet whose key the worker doesn't
///                           hold.
///
/// Run:
///   forge script script/Deploy.s.sol:DeployPOOOL \
///     --rpc-url $POLYGON_AMOY_RPC_URL \
///     --broadcast --verify
contract DeployPOOOL is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("DEPLOYER_PRIVATE_KEY");
        address adminAddress = vm.envAddress("ADMIN_ADDRESS");

        vm.startBroadcast(deployerPrivateKey);

        POOOLAssetToken implementation = new POOOLAssetToken();
        IdentityRegistry identityRegistry = new IdentityRegistry(adminAddress);
        AssetFactory factory = new AssetFactory(
            adminAddress,
            address(implementation),
            address(identityRegistry)
        );

        // DEPLOYER_ROLE on the factory governs who can call deployAsset().
        // The constructor already grants this to `adminAddress`; this
        // line is redundant but kept for clarity / future co-admins.
        factory.grantRole(factory.DEPLOYER_ROLE(), adminAddress);

        vm.stopBroadcast();
    }
}
