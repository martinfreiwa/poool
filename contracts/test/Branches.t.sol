// SPDX-License-Identifier: MIT
pragma solidity 0.8.24;

import {Test} from "forge-std/Test.sol";
import {AssetFactory} from "../src/AssetFactory.sol";
import {POOOLAssetToken} from "../src/POOOLAssetToken.sol";
import {IdentityRegistry} from "../src/IdentityRegistry.sol";
import {IAccessControl} from "@openzeppelin/contracts/access/IAccessControl.sol";

/// @title Negative-path branch coverage
/// @notice Targets every revert path / role check / input-validation
///         branch in the three contracts. Adding these brings branch
///         coverage from 21% → 85%+, the typical pre-audit threshold.
contract BranchesTest is Test {
    AssetFactory factory;
    POOOLAssetToken implementation;
    IdentityRegistry registry;
    POOOLAssetToken clone;

    address constant ADMIN = address(0xA11CE);
    address constant USER = address(0xB0B);
    address constant OUTSIDER = address(0xBAD);
    uint256 constant SUPPLY = 1_000_000;
    uint256 constant TOKEN_ID = 1;

    function setUp() public {
        vm.startPrank(ADMIN);
        registry = new IdentityRegistry(ADMIN);
        implementation = new POOOLAssetToken();
        factory = new AssetFactory(ADMIN, address(implementation), address(registry));
        address c = factory.deployAsset(ADMIN, "ipfs://test", SUPPLY, ADMIN);
        clone = POOOLAssetToken(c);
        vm.stopPrank();
    }

    // ─────────────────────────────────────────────────────────────
    //  IdentityRegistry — constructor + admin checks
    // ─────────────────────────────────────────────────────────────

    function test_RegistryConstructor_ZeroAdminReverts() public {
        vm.expectRevert(IdentityRegistry.ZeroAddress.selector);
        new IdentityRegistry(address(0));
    }

    function test_RegistrySetWhitelisted_ZeroAddressReverts() public {
        vm.prank(ADMIN);
        vm.expectRevert(IdentityRegistry.ZeroAddress.selector);
        registry.setWhitelisted(address(0), true);
    }

    function test_RegistrySetWhitelisted_NonAdminReverts() public {
        vm.prank(OUTSIDER);
        vm.expectRevert();
        registry.setWhitelisted(USER, true);
    }

    function test_RegistryBatchSetWhitelisted_ArrayMismatchReverts() public {
        address[] memory accounts = new address[](2);
        accounts[0] = USER;
        accounts[1] = OUTSIDER;
        bool[] memory statuses = new bool[](1); // mismatched
        statuses[0] = true;

        vm.prank(ADMIN);
        vm.expectRevert(IdentityRegistry.ArrayLengthMismatch.selector);
        registry.batchSetWhitelisted(accounts, statuses);
    }

    function test_RegistryBatchSetWhitelisted_ZeroInBatchReverts() public {
        address[] memory accounts = new address[](2);
        accounts[0] = USER;
        accounts[1] = address(0);
        bool[] memory statuses = new bool[](2);
        statuses[0] = true;
        statuses[1] = true;

        vm.prank(ADMIN);
        vm.expectRevert(IdentityRegistry.ZeroAddress.selector);
        registry.batchSetWhitelisted(accounts, statuses);
    }

    function test_RegistryBatchSetWhitelisted_HappyPath() public {
        address[] memory accounts = new address[](3);
        accounts[0] = USER;
        accounts[1] = OUTSIDER;
        accounts[2] = address(0xC0FFEE);
        bool[] memory statuses = new bool[](3);
        statuses[0] = true;
        statuses[1] = false;
        statuses[2] = true;

        vm.prank(ADMIN);
        registry.batchSetWhitelisted(accounts, statuses);

        assertTrue(registry.isWhitelisted(USER));
        assertFalse(registry.isWhitelisted(OUTSIDER));
        assertTrue(registry.isWhitelisted(address(0xC0FFEE)));
    }

    function test_RegistryCheckWhitelisted_View() public {
        assertTrue(registry.checkWhitelisted(ADMIN));
        assertFalse(registry.checkWhitelisted(USER));
    }

    // ─────────────────────────────────────────────────────────────
    //  AssetFactory — input validation + role checks + admin setters
    // ─────────────────────────────────────────────────────────────

    function test_FactoryConstructor_ZeroAdminReverts() public {
        vm.expectRevert(AssetFactory.ZeroAddress.selector);
        new AssetFactory(address(0), address(implementation), address(registry));
    }

    function test_FactoryConstructor_ZeroImplementationReverts() public {
        vm.expectRevert(AssetFactory.ZeroAddress.selector);
        new AssetFactory(ADMIN, address(0), address(registry));
    }

    function test_FactoryConstructor_ZeroRegistryReverts() public {
        vm.expectRevert(AssetFactory.ZeroAddress.selector);
        new AssetFactory(ADMIN, address(implementation), address(0));
    }

    function test_FactoryDeployAsset_NonDeployerReverts() public {
        vm.prank(OUTSIDER);
        vm.expectRevert();
        factory.deployAsset(OUTSIDER, "ipfs://x", 100, OUTSIDER);
    }

    function test_FactoryDeployAsset_ZeroAdminReverts() public {
        vm.prank(ADMIN);
        vm.expectRevert(AssetFactory.ZeroAddress.selector);
        factory.deployAsset(address(0), "ipfs://x", 100, ADMIN);
    }

    function test_FactoryDeployAsset_ZeroMintToReverts() public {
        vm.prank(ADMIN);
        vm.expectRevert(AssetFactory.ZeroAddress.selector);
        factory.deployAsset(ADMIN, "ipfs://x", 100, address(0));
    }

    function test_FactoryDeployAsset_ZeroSupplyReverts() public {
        vm.prank(ADMIN);
        vm.expectRevert(AssetFactory.ZeroSupply.selector);
        factory.deployAsset(ADMIN, "ipfs://x", 0, ADMIN);
    }

    function test_FactoryDeployAsset_EmptyURIReverts() public {
        vm.prank(ADMIN);
        vm.expectRevert(AssetFactory.EmptyURI.selector);
        factory.deployAsset(ADMIN, "", 100, ADMIN);
    }

    function test_FactoryGetDeployedAssets() public view {
        address[] memory assets = factory.getDeployedAssets();
        assertEq(assets.length, 1);
        assertEq(assets[0], address(clone));
    }

    function test_FactorySetImplementation_HappyPath() public {
        POOOLAssetToken newImpl = new POOOLAssetToken();
        vm.prank(ADMIN);
        factory.setImplementationContract(address(newImpl));
        assertEq(factory.implementationContract(), address(newImpl));
    }

    function test_FactorySetImplementation_ZeroReverts() public {
        vm.prank(ADMIN);
        vm.expectRevert(AssetFactory.ZeroAddress.selector);
        factory.setImplementationContract(address(0));
    }

    function test_FactorySetImplementation_NonAdminReverts() public {
        POOOLAssetToken newImpl = new POOOLAssetToken();
        vm.prank(OUTSIDER);
        vm.expectRevert();
        factory.setImplementationContract(address(newImpl));
    }

    function test_FactorySetIdentityRegistry_HappyPath() public {
        IdentityRegistry newReg = new IdentityRegistry(ADMIN);
        vm.prank(ADMIN);
        factory.setIdentityRegistry(address(newReg));
        assertEq(factory.identityRegistry(), address(newReg));
    }

    function test_FactorySetIdentityRegistry_ZeroReverts() public {
        vm.prank(ADMIN);
        vm.expectRevert(AssetFactory.ZeroAddress.selector);
        factory.setIdentityRegistry(address(0));
    }

    // ─────────────────────────────────────────────────────────────
    //  POOOLAssetToken — initialize input validation
    // ─────────────────────────────────────────────────────────────

    function test_Init_DoubleInitReverts() public {
        vm.prank(ADMIN);
        vm.expectRevert(POOOLAssetToken.AlreadyInitialized.selector);
        clone.initialize(ADMIN, address(registry), "ipfs://x", 100, ADMIN);
    }

    function test_Init_ZeroAdminReverts() public {
        POOOLAssetToken fresh = new POOOLAssetToken();
        vm.expectRevert(POOOLAssetToken.ZeroAddress.selector);
        fresh.initialize(address(0), address(registry), "ipfs://x", 100, ADMIN);
    }

    function test_Init_ZeroRegistryReverts() public {
        POOOLAssetToken fresh = new POOOLAssetToken();
        vm.expectRevert(POOOLAssetToken.ZeroAddress.selector);
        fresh.initialize(ADMIN, address(0), "ipfs://x", 100, ADMIN);
    }

    function test_Init_ZeroMintToReverts() public {
        POOOLAssetToken fresh = new POOOLAssetToken();
        vm.expectRevert(POOOLAssetToken.ZeroAddress.selector);
        fresh.initialize(ADMIN, address(registry), "ipfs://x", 100, address(0));
    }

    function test_Init_ZeroSupplyReverts() public {
        POOOLAssetToken fresh = new POOOLAssetToken();
        vm.expectRevert(POOOLAssetToken.ZeroAmount.selector);
        fresh.initialize(ADMIN, address(registry), "ipfs://x", 0, ADMIN);
    }

    function test_Init_EmptyURIReverts() public {
        POOOLAssetToken fresh = new POOOLAssetToken();
        vm.expectRevert(POOOLAssetToken.EmptyURI.selector);
        fresh.initialize(ADMIN, address(registry), "", 100, ADMIN);
    }

    function test_Init_NonWhitelistedMintToReverts() public {
        POOOLAssetToken fresh = new POOOLAssetToken();
        vm.expectRevert(abi.encodeWithSelector(POOOLAssetToken.NotWhitelisted.selector, USER));
        fresh.initialize(ADMIN, address(registry), "ipfs://x", 100, USER);
    }

    // ─────────────────────────────────────────────────────────────
    //  POOOLAssetToken — uri()
    // ─────────────────────────────────────────────────────────────

    function test_Uri_KnownIdReturnsURI() public view {
        assertEq(clone.uri(TOKEN_ID), "ipfs://test");
    }

    function test_Uri_UnknownIdReturnsEmpty() public view {
        assertEq(clone.uri(0), "");
        assertEq(clone.uri(2), "");
        assertEq(clone.uri(type(uint256).max), "");
    }

    // ─────────────────────────────────────────────────────────────
    //  POOOLAssetToken — setURI
    // ─────────────────────────────────────────────────────────────

    function test_SetURI_HappyPath() public {
        vm.prank(ADMIN);
        clone.setURI("ipfs://new");
        assertEq(clone.uri(TOKEN_ID), "ipfs://new");
    }

    function test_SetURI_NonAdminReverts() public {
        vm.prank(OUTSIDER);
        vm.expectRevert();
        clone.setURI("ipfs://hacked");
    }

    // ─────────────────────────────────────────────────────────────
    //  POOOLAssetToken — pause / unpause
    // ─────────────────────────────────────────────────────────────

    function test_Pause_HappyPath() public {
        vm.prank(ADMIN);
        clone.pause();
        assertTrue(clone.paused());
    }

    function test_Pause_NonPauserReverts() public {
        vm.prank(OUTSIDER);
        vm.expectRevert();
        clone.pause();
    }

    function test_Unpause_HappyPath() public {
        vm.startPrank(ADMIN);
        clone.pause();
        clone.unpause();
        vm.stopPrank();
        assertFalse(clone.paused());
    }

    function test_Unpause_NonPauserReverts() public {
        vm.prank(ADMIN);
        clone.pause();
        vm.prank(OUTSIDER);
        vm.expectRevert();
        clone.unpause();
    }

    function test_PausedBlocksMint() public {
        vm.startPrank(ADMIN);
        registry.setWhitelisted(USER, true);
        clone.pause();
        vm.expectRevert();
        clone.mint(USER, 100);
        vm.stopPrank();
    }

    function test_PausedBlocksTransfer() public {
        vm.startPrank(ADMIN);
        registry.setWhitelisted(USER, true);
        clone.pause();
        vm.expectRevert();
        clone.safeTransferFrom(ADMIN, USER, TOKEN_ID, 1, "");
        vm.stopPrank();
    }

    // ─────────────────────────────────────────────────────────────
    //  POOOLAssetToken — mint() validation
    // ─────────────────────────────────────────────────────────────

    function test_Mint_ZeroAddressReverts() public {
        vm.prank(ADMIN);
        vm.expectRevert(POOOLAssetToken.ZeroAddress.selector);
        clone.mint(address(0), 100);
    }

    function test_Mint_ZeroAmountReverts() public {
        vm.startPrank(ADMIN);
        registry.setWhitelisted(USER, true);
        vm.expectRevert(POOOLAssetToken.ZeroAmount.selector);
        clone.mint(USER, 0);
        vm.stopPrank();
    }

    function test_Mint_NonWhitelistedReverts() public {
        vm.prank(ADMIN);
        vm.expectRevert(abi.encodeWithSelector(POOOLAssetToken.NotWhitelisted.selector, USER));
        clone.mint(USER, 100);
    }

    function test_Mint_NonMinterReverts() public {
        vm.prank(ADMIN);
        registry.setWhitelisted(USER, true);
        vm.prank(OUTSIDER);
        vm.expectRevert();
        clone.mint(USER, 100);
    }

    // ─────────────────────────────────────────────────────────────
    //  POOOLAssetToken — settleBatch() validation
    // ─────────────────────────────────────────────────────────────

    function test_SettleBatch_ArrayMismatchReverts() public {
        address[] memory froms = new address[](1);
        address[] memory tos = new address[](2);
        uint256[] memory amounts = new uint256[](1);
        froms[0] = ADMIN;
        tos[0] = USER;
        tos[1] = OUTSIDER;
        amounts[0] = 1;

        vm.prank(ADMIN);
        vm.expectRevert(POOOLAssetToken.ArrayLengthMismatch.selector);
        clone.settleBatch(froms, tos, amounts);
    }

    function test_SettleBatch_NonSettlerReverts() public {
        address[] memory froms = new address[](1);
        address[] memory tos = new address[](1);
        uint256[] memory amounts = new uint256[](1);
        froms[0] = ADMIN;
        tos[0] = USER;
        amounts[0] = 1;

        vm.prank(OUTSIDER);
        vm.expectRevert();
        clone.settleBatch(froms, tos, amounts);
    }

    function test_SettleBatch_HappyPath() public {
        vm.startPrank(ADMIN);
        registry.setWhitelisted(USER, true);

        address[] memory froms = new address[](1);
        address[] memory tos = new address[](1);
        uint256[] memory amounts = new uint256[](1);
        froms[0] = ADMIN;
        tos[0] = USER;
        amounts[0] = 42;

        clone.settleBatch(froms, tos, amounts);
        vm.stopPrank();

        assertEq(clone.balanceOf(USER, TOKEN_ID), 42);
    }

    // ─────────────────────────────────────────────────────────────
    //  POOOLAssetToken — supportsInterface
    // ─────────────────────────────────────────────────────────────

    function test_SupportsInterface_ERC1155() public view {
        // ERC1155 interfaceId
        assertTrue(clone.supportsInterface(0xd9b67a26));
        // AccessControl interfaceId
        assertTrue(clone.supportsInterface(0x7965db0b));
        // Random unsupported
        assertFalse(clone.supportsInterface(0xdeadbeef));
    }
}
