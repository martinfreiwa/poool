// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Script} from "forge-std/Script.sol";
import {POOOLAssetToken} from "../src/POOOLAssetToken.sol";
import {AssetFactory} from "../src/AssetFactory.sol";

contract CreateImplementation is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        address factoryAddress = vm.envAddress("FACTORY_ADDRESS");

        vm.startBroadcast(deployerPrivateKey);

        // 1. Deploy fresh implementation
        POOOLAssetToken impl = new POOOLAssetToken();
        
        // 2. Point factory to new implementation
        AssetFactory factory = AssetFactory(factoryAddress);
        factory.setImplementationContract(address(impl));

        vm.stopBroadcast();
    }
}
