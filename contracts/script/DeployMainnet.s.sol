// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Script} from "forge-std/Script.sol";
import {AssetFactory} from "../src/AssetFactory.sol";
import {IdentityRegistry} from "../src/IdentityRegistry.sol";
import {POOOLAssetToken} from "../src/POOOLAssetToken.sol";

/// Polygon mainnet deploy: fresh implementation + IdentityRegistry + AssetFactory.
/// Differs from Deploy.s.sol which hardcodes the Amoy implementation address.
contract DeployMainnet is Script {
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
        factory.grantRole(factory.DEPLOYER_ROLE(), adminAddress);

        vm.stopBroadcast();
    }
}
