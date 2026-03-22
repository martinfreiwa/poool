// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {AssetFactory} from "../src/AssetFactory.sol";
import {POOOLAssetToken} from "../src/POOOLAssetToken.sol";
import {IdentityRegistry} from "../src/IdentityRegistry.sol";

contract POOOLAssetTokenFuzzTest is Test {
    AssetFactory public factory;
    POOOLAssetToken public implementation;
    IdentityRegistry public identityRegistry;
    POOOLAssetToken public clone;

    address public admin = address(1);

    function setUp() public {
        vm.startPrank(admin);
        identityRegistry = new IdentityRegistry(admin);
        implementation = new POOOLAssetToken();
        factory = new AssetFactory(admin, address(implementation), address(identityRegistry));
        
        address cloneAddr = factory.deployAsset(admin, "ipfs://fuzz", 1_000_000, admin);
        clone = POOOLAssetToken(cloneAddr);
        vm.stopPrank();
    }

    function testFuzz_Whitelisting(address randomUser) public {
        // Assume randomUser is not the admin or 0 and is an EOA
        vm.assume(randomUser != address(0) && randomUser != admin);
        vm.assume(randomUser.code.length == 0);

        vm.startPrank(admin);
        
        // Setup initial user
        identityRegistry.setWhitelisted(randomUser, true);
        clone.safeTransferFrom(admin, randomUser, 1, 80, "");
        
        // Remove whitelist
        identityRegistry.setWhitelisted(randomUser, false);
        
        // Any transfer TO randomUser should now fail
        vm.expectRevert();
        clone.safeTransferFrom(admin, randomUser, 1, 10, "");
        
        // But burn from standard rules is allowed, although safeTransferFrom without receiver doesn't burn.
        // Wait, _update allows burns to address(0) without whitelisting.
        
        vm.stopPrank();
    }

    function testFuzz_MaxOwnership(uint256 mintAmount, uint256 transferAmount) public {
        vm.assume(mintAmount > 0 && mintAmount < 1e12); // reasonable max
        address randomUser = address(123);
        
        vm.startPrank(admin);
        identityRegistry.setWhitelisted(randomUser, true);
        
        address cloneAddr = factory.deployAsset(admin, "ipfs://fuzz2", mintAmount, admin);
        POOOLAssetToken specificClone = POOOLAssetToken(cloneAddr);
        
        uint256 maxAllowed = (mintAmount * specificClone.MAX_OWNERSHIP_BPS()) / specificClone.MAX_BPS();
        
        // Ensure mintAmount is large enough so maxAllowed + 1 <= mintAmount
        vm.assume(maxAllowed + 1 <= mintAmount);
        
        transferAmount = bound(transferAmount, maxAllowed + 1, mintAmount);
        
        vm.expectRevert();
        specificClone.safeTransferFrom(admin, randomUser, 1, transferAmount, "");
        
        vm.stopPrank();
    }
}
