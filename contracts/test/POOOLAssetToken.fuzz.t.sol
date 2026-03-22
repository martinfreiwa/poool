// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {AssetFactory} from "../src/AssetFactory.sol";
import {POOOLAssetToken} from "../src/POOOLAssetToken.sol";
import {IdentityRegistry} from "../src/IdentityRegistry.sol";

/**
 * @title POOOLAssetTokenFuzzTest — Phase 11.5
 * @notice Comprehensive fuzz & invariant tests for the POOOL ERC-1155 token.
 *
 * Tests validate:
 *   1. Supply conservation (total supply never changes outside mint/burn)
 *   2. KYC whitelist enforcement under random inputs
 *   3. Max ownership cap (80%) under random transfer amounts
 *   4. SettleBatch correctness with random batch sizes
 *   5. Pause/Unpause isolation
 *   6. Double initialization prevention
 */
contract POOOLAssetTokenFuzzTest is Test {
    AssetFactory public factory;
    POOOLAssetToken public implementation;
    IdentityRegistry public identityRegistry;
    POOOLAssetToken public clone;

    address public admin = address(1);
    uint256 public constant INITIAL_SUPPLY = 1_000_000;
    uint256 public constant TOKEN_ID = 1;

    function setUp() public {
        vm.startPrank(admin);
        identityRegistry = new IdentityRegistry(admin);
        implementation = new POOOLAssetToken();
        factory = new AssetFactory(admin, address(implementation), address(identityRegistry));

        address cloneAddr = factory.deployAsset(admin, "ipfs://test", INITIAL_SUPPLY, admin);
        clone = POOOLAssetToken(cloneAddr);
        vm.stopPrank();
    }

    // ─── 1. Supply Conservation ────────────────────────────────

    function testFuzz_SupplyConservation(uint256 transferAmount) public {
        transferAmount = bound(transferAmount, 1, INITIAL_SUPPLY * 80 / 100);
        address user = address(42);

        vm.startPrank(admin);
        identityRegistry.setWhitelisted(user, true);

        uint256 supplyBefore = clone.totalSupply(TOKEN_ID);
        clone.safeTransferFrom(admin, user, TOKEN_ID, transferAmount, "");
        uint256 supplyAfter = clone.totalSupply(TOKEN_ID);

        // Supply must be unchanged after a transfer (not mint/burn)
        assertEq(supplyBefore, supplyAfter, "Supply changed during transfer");

        // Conservation: admin + user = total supply
        uint256 adminBal = clone.balanceOf(admin, TOKEN_ID);
        uint256 userBal = clone.balanceOf(user, TOKEN_ID);
        assertEq(adminBal + userBal, INITIAL_SUPPLY, "Token balance mismatch");

        vm.stopPrank();
    }

    // ─── 2. KYC Whitelist Enforcement ──────────────────────────

    function testFuzz_NonWhitelistedRejected(address randomUser) public {
        vm.assume(randomUser != address(0) && randomUser != admin);
        vm.assume(randomUser.code.length == 0);

        // Do NOT whitelist — transfer should revert
        vm.startPrank(admin);
        vm.expectRevert();
        clone.safeTransferFrom(admin, randomUser, TOKEN_ID, 1, "");
        vm.stopPrank();
    }

    function testFuzz_WhitelistToggle(address randomUser, uint256 amount) public {
        vm.assume(randomUser != address(0) && randomUser != admin);
        vm.assume(randomUser.code.length == 0);
        amount = bound(amount, 1, INITIAL_SUPPLY * 80 / 100);

        vm.startPrank(admin);

        // Step 1: Whitelist and transfer
        identityRegistry.setWhitelisted(randomUser, true);
        clone.safeTransferFrom(admin, randomUser, TOKEN_ID, amount, "");
        assertEq(clone.balanceOf(randomUser, TOKEN_ID), amount);

        // Step 2: Remove whitelist — further transfers TO user should fail
        identityRegistry.setWhitelisted(randomUser, false);
        vm.expectRevert();
        clone.safeTransferFrom(admin, randomUser, TOKEN_ID, 1, "");

        vm.stopPrank();
    }

    // ─── 3. Max Ownership Cap (80%) ────────────────────────────

    function testFuzz_MaxOwnershipEnforcement(uint256 amount) public {
        address user = address(42);
        uint256 maxAllowed = (INITIAL_SUPPLY * 80) / 100; // 800,000

        // Attempt to transfer more than 80%
        amount = bound(amount, maxAllowed + 1, INITIAL_SUPPLY);

        vm.startPrank(admin);
        identityRegistry.setWhitelisted(user, true);

        vm.expectRevert();
        clone.safeTransferFrom(admin, user, TOKEN_ID, amount, "");
        vm.stopPrank();
    }

    function testFuzz_ExactMaxOwnershipAllowed(uint256 supply) public {
        supply = bound(supply, 100, 1e12);
        address user = address(42);

        vm.startPrank(admin);
        identityRegistry.setWhitelisted(user, true);

        // Deploy a new clone with this supply
        address cloneAddr = factory.deployAsset(admin, "ipfs://cap-test", supply, admin);
        POOOLAssetToken c = POOOLAssetToken(cloneAddr);

        uint256 maxAllowed = (supply * c.MAX_OWNERSHIP_BPS()) / c.MAX_BPS();

        // Transfer exactly maxAllowed should succeed
        if (maxAllowed > 0 && maxAllowed <= supply) {
            c.safeTransferFrom(admin, user, TOKEN_ID, maxAllowed, "");
            assertEq(c.balanceOf(user, TOKEN_ID), maxAllowed);
        }

        vm.stopPrank();
    }

    // ─── 4. SettleBatch Correctness ────────────────────────────

    function testFuzz_SettleBatchCorrectness(uint8 batchSize) public {
        batchSize = uint8(bound(batchSize, 1, 10));

        vm.startPrank(admin);

        // Create whitelisted users
        address[] memory froms = new address[](batchSize);
        address[] memory tos = new address[](batchSize);
        uint256[] memory amounts = new uint256[](batchSize);

        // Give each user some tokens first
        uint256 perUser = 100;
        for (uint256 i = 0; i < batchSize; i++) {
            address user = address(uint160(100 + i));
            identityRegistry.setWhitelisted(user, true);
            clone.safeTransferFrom(admin, user, TOKEN_ID, perUser, "");

            address recipient = address(uint160(200 + i));
            identityRegistry.setWhitelisted(recipient, true);

            froms[i] = user;
            tos[i] = recipient;
            amounts[i] = 10;
        }

        uint256 supplyBefore = clone.totalSupply(TOKEN_ID);

        // Execute batch settlement
        clone.settleBatch(froms, tos, amounts);

        uint256 supplyAfter = clone.totalSupply(TOKEN_ID);
        assertEq(supplyBefore, supplyAfter, "Supply changed during settlement");

        // Verify each recipient got their tokens
        for (uint256 i = 0; i < batchSize; i++) {
            assertEq(clone.balanceOf(tos[i], TOKEN_ID), 10, "Recipient didn't receive tokens");
            assertEq(clone.balanceOf(froms[i], TOKEN_ID), perUser - 10, "Sender balance wrong");
        }

        vm.stopPrank();
    }

    function testFuzz_SettleBatchArrayMismatch(uint8 extraLen) public {
        extraLen = uint8(bound(extraLen, 1, 5));

        vm.startPrank(admin);

        address[] memory froms = new address[](1);
        address[] memory tos = new address[](1 + extraLen);
        uint256[] memory amounts = new uint256[](1);

        froms[0] = admin;
        tos[0] = address(42);
        amounts[0] = 1;

        vm.expectRevert();
        clone.settleBatch(froms, tos, amounts);
        vm.stopPrank();
    }

    // ─── 5. Pause Isolation ────────────────────────────────────

    function testFuzz_PausedBlocksTransfer(uint256 amount) public {
        amount = bound(amount, 1, INITIAL_SUPPLY * 80 / 100);
        address user = address(42);

        vm.startPrank(admin);
        identityRegistry.setWhitelisted(user, true);

        // Pause
        clone.pause();

        // All transfers should fail
        vm.expectRevert();
        clone.safeTransferFrom(admin, user, TOKEN_ID, amount, "");

        // Unpause and verify transfer works
        clone.unpause();
        clone.safeTransferFrom(admin, user, TOKEN_ID, amount, "");
        assertEq(clone.balanceOf(user, TOKEN_ID), amount);

        vm.stopPrank();
    }

    // ─── 6. Double Init Prevention ─────────────────────────────

    function testFuzz_DoubleInitReverts(uint256 supply) public {
        supply = bound(supply, 1, 1e12);

        vm.startPrank(admin);

        // The clone is already initialized — second init must revert
        vm.expectRevert();
        clone.initialize(admin, address(identityRegistry), "ipfs://dup", supply, admin);

        vm.stopPrank();
    }

    // ─── 7. Settlement Role Enforcement ────────────────────────

    function testFuzz_NonSettlerCannotBatchSettle(address randomCaller) public {
        vm.assume(randomCaller != admin && randomCaller != address(0));

        address[] memory froms = new address[](1);
        address[] memory tos = new address[](1);
        uint256[] memory amounts = new uint256[](1);

        froms[0] = admin;
        tos[0] = address(42);
        amounts[0] = 1;

        vm.prank(randomCaller);
        vm.expectRevert();
        clone.settleBatch(froms, tos, amounts);
    }
}
