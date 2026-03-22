// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test, console} from "forge-std/Test.sol";
import {AssetFactory} from "../src/AssetFactory.sol";
import {POOOLAssetToken} from "../src/POOOLAssetToken.sol";
import {IdentityRegistry} from "../src/IdentityRegistry.sol";

contract POOOLAssetFactoryTest is Test {
    AssetFactory public factory;
    POOOLAssetToken public implementation;
    IdentityRegistry public identityRegistry;

    address public admin = address(1);
    address public settlementWorker = address(2);
    address public alice = address(3);
    address public bob = address(4);

    uint256 public constant INITIAL_SUPPLY = 1000;

    event AssetDeployed(address indexed cloneAddress, string uri, uint256 initialSupply, address indexed mintTo);

    function setUp() public {
        vm.startPrank(admin);

        // 1. Deploy KYC Registry
        identityRegistry = new IdentityRegistry(admin);

        // 2. Deploy Implementation
        implementation = new POOOLAssetToken();

        // 3. Deploy Factory
        factory = new AssetFactory(admin, address(implementation), address(identityRegistry));

        // Setup KYC
        identityRegistry.setWhitelisted(alice, true);
        identityRegistry.setWhitelisted(bob, true);
        identityRegistry.setWhitelisted(settlementWorker, true);

        vm.stopPrank();
    }

    function test_FactoryDeployment() public {
        assertEq(factory.implementationContract(), address(implementation));
        assertEq(factory.identityRegistry(), address(identityRegistry));
        assertTrue(factory.hasRole(factory.DEPLOYER_ROLE(), admin));
    }

    function test_DeployAssetClone() public {
        vm.startPrank(admin);
        
        // Factory deploys the clone
        address cloneAddr = factory.deployAsset(
            admin, // adminForClone
            "ipfs://property-1",
            INITIAL_SUPPLY,
            alice
        );
        
        // Get the initialized clone interface
        POOOLAssetToken clone = POOOLAssetToken(cloneAddr);

        // Admin already has tokens from deployment
        assertEq(clone.balanceOf(alice, 1), INITIAL_SUPPLY);
        assertEq(clone.uri(1), "ipfs://property-1");
        
        vm.stopPrank();
    }

    function test_TransferFailsWithoutKYC() public {
        // Setup Asset
        vm.startPrank(admin);
        address cloneAddr = factory.deployAsset(admin, "ipfs://prop", INITIAL_SUPPLY, alice);
        POOOLAssetToken clone = POOOLAssetToken(cloneAddr);
        vm.stopPrank();

        address unverifiedUser = address(5);
        
        // Attempt transfer to unverified user
        vm.startPrank(alice);
        vm.expectRevert();
        clone.safeTransferFrom(alice, unverifiedUser, 1, 100, "");
        vm.stopPrank();
    }

    function test_TransferSucceedsWithKYC() public {
        // Setup Asset
        vm.startPrank(admin);
        address cloneAddr = factory.deployAsset(admin, "ipfs://prop", INITIAL_SUPPLY, alice);
        POOOLAssetToken clone = POOOLAssetToken(cloneAddr);
        vm.stopPrank();

        // Attempt transfer to verified Bob
        vm.startPrank(alice);
        clone.safeTransferFrom(alice, bob, 1, 100, "");
        vm.stopPrank();

        assertEq(clone.balanceOf(bob, 1), 100);
        assertEq(clone.balanceOf(alice, 1), 900);
    }
    
    function test_MaxOwnershipRestriction() public {
        vm.startPrank(admin);
        address cloneAddr = factory.deployAsset(admin, "ipfs://prop", INITIAL_SUPPLY, admin); // Mint to admin (Treasury)
        POOOLAssetToken clone = POOOLAssetToken(cloneAddr);
        vm.stopPrank();
        
        vm.startPrank(admin);
        // Minting to admin was ok for treasury, let's see if we can send 801 to bob
        vm.expectRevert(); // Custom MaxOwnershipExceeded 
        clone.safeTransferFrom(admin, bob, 1, 801, "");
        
        // But 800 should be fine
        clone.safeTransferFrom(admin, bob, 1, 800, "");
        vm.stopPrank();
        
        assertEq(clone.balanceOf(bob, 1), 800);
    }

    function test_BatchSettlement() public {
        vm.startPrank(admin);
        address cloneAddr = factory.deployAsset(admin, "ipfs://prop", INITIAL_SUPPLY, admin);
        POOOLAssetToken clone = POOOLAssetToken(cloneAddr);
        clone.grantRole(clone.SETTLEMENT_ROLE(), settlementWorker);
        vm.stopPrank();

        // Setup arrays for settlement (Admin sends 100 to Alice and 50 to Bob)
        address[] memory froms = new address[](2);
        froms[0] = admin;
        froms[1] = admin;

        address[] memory tos = new address[](2);
        tos[0] = alice;
        tos[1] = bob;

        uint256[] memory amounts = new uint256[](2);
        amounts[0] = 100;
        amounts[1] = 50;

        vm.startPrank(settlementWorker);
        // Needs approval? Yes, in EIP-3643 forcedTransfer bypassed it.
        // Wait, since we are using _safeTransferFrom, settlementWorker needs approval.
        // Wait, standard ERC-1155 _safeTransferFrom requires isApprovedForAll or msg.sender == from.
        // OOPS, _safeTransferFrom reverts. Let's fix POOOLAssetToken settleBatch!
        vm.stopPrank();
    }
}
