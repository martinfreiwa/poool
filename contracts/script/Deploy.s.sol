// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Script} from "forge-std/Script.sol";
import {AssetFactory} from "../src/AssetFactory.sol";
import {IdentityRegistry} from "../src/IdentityRegistry.sol";
import {POOOLAssetToken} from "../src/POOOLAssetToken.sol";

contract DeployPOOOL is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("DEPLOYER_PRIVATE_KEY");
        address adminAddress = vm.envAddress("ADMIN_ADDRESS");
        
        vm.startBroadcast(deployerPrivateKey);

        // 1. Deploy IdentityRegistry
        IdentityRegistry identityRegistry = new IdentityRegistry(adminAddress);
        
        // 2. We ALREADY have the implementation token deployed:
        address implementation = 0xb61CCe33B546a5C7c36F0B58119e7F4B3D1D04e5;
        
        // 3. Deploy AssetFactory
        AssetFactory factory = new AssetFactory(
            adminAddress,
            implementation,
            address(identityRegistry)
        );

        // 4. Grant roles
        factory.grantRole(factory.DEPLOYER_ROLE(), adminAddress);
        
        vm.stopBroadcast();
    }
}
