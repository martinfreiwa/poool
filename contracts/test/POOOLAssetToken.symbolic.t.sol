// SPDX-License-Identifier: MIT
pragma solidity 0.8.24;

import {Test} from "forge-std/Test.sol";
import {AssetFactory} from "../src/AssetFactory.sol";
import {POOOLAssetToken} from "../src/POOOLAssetToken.sol";
import {IdentityRegistry} from "../src/IdentityRegistry.sol";

/// @title POOOL — Halmos symbolic checks
/// @notice Symbolic-execution-friendly properties (`check_*` prefix) that
///         Halmos converts into mathematical proofs over symbolic input
///         spaces. Foundry runs these as concrete fuzz too.
///
///         Run:  halmos --contract POOOLSymbolicTest
contract POOOLSymbolicTest is Test {
    AssetFactory factory;
    POOOLAssetToken implementation;
    IdentityRegistry registry;
    POOOLAssetToken clone;

    address constant ADMIN = address(0xA11CE);
    uint256 constant SUPPLY = 1_000_000;

    function setUp() public {
        vm.startPrank(ADMIN);
        registry = new IdentityRegistry(ADMIN);
        implementation = new POOOLAssetToken();
        factory = new AssetFactory(ADMIN, address(implementation), address(registry));
        address cloneAddr = factory.deployAsset(ADMIN, "ipfs://test", SUPPLY, ADMIN);
        clone = POOOLAssetToken(cloneAddr);
        vm.stopPrank();
    }

    /// Property 1 — Total supply is exactly the initial mint after deploy.
    /// Symbolic constants only; trivially true but exercises the
    /// initialization path.
    function check_TotalSupplyAfterInit() public view {
        assert(clone.totalSupply(1) == SUPPLY);
    }

    /// Property 2 — A non-whitelisted recipient can never receive a
    /// transfer. Symbolic over `to` and `amount`.
    function check_NonWhitelistedCannotReceive(address to, uint256 amount) public {
        vm.assume(to != address(0));
        vm.assume(to != ADMIN);              // ADMIN is whitelisted
        vm.assume(amount > 0 && amount <= SUPPLY);
        vm.assume(!registry.isWhitelisted(to));

        vm.prank(ADMIN);
        try clone.safeTransferFrom(ADMIN, to, 1, amount, "") {
            assert(false); // must have reverted
        } catch {
            // expected
        }
    }

    /// Property 3 — Pausing blocks all transfers regardless of role.
    function check_PausedBlocksTransfer(address to, uint256 amount) public {
        vm.assume(to != address(0) && to != ADMIN);
        vm.assume(amount > 0 && amount <= SUPPLY);

        vm.startPrank(ADMIN);
        registry.setWhitelisted(to, true);
        clone.pause();
        vm.stopPrank();

        vm.prank(ADMIN);
        try clone.safeTransferFrom(ADMIN, to, 1, amount, "") {
            assert(false);
        } catch {}
    }

    /// Property 4 — `mint(to, amount)` always increases `to`'s balance
    /// by exactly `amount` (when not paused, recipient whitelisted,
    /// caller has MINTER_ROLE, amount > 0).
    function check_MintIncreasesBalanceExactly(address to, uint256 amount) public {
        vm.assume(to != address(0) && to != ADMIN);
        vm.assume(amount > 0 && amount < type(uint128).max);

        vm.startPrank(ADMIN);
        registry.setWhitelisted(to, true);
        uint256 balanceBefore = clone.balanceOf(to, 1);

        // Skip if would exceed 80% cap (separate property covers the cap).
        uint256 supplyAfter = clone.totalSupply(1) + amount;
        uint256 maxAllowed = (supplyAfter * 8000) / 10000;
        vm.assume(balanceBefore + amount <= maxAllowed);

        clone.mint(to, amount);
        vm.stopPrank();

        assert(clone.balanceOf(to, 1) == balanceBefore + amount);
    }

    /// Property 5 — Re-initializing a clone always reverts.
    function check_DoubleInitReverts(address newAdmin, uint256 newSupply) public {
        vm.assume(newAdmin != address(0));
        vm.assume(newSupply > 0);

        vm.prank(ADMIN);
        try clone.initialize(newAdmin, address(registry), "ipfs://x", newSupply, newAdmin) {
            assert(false);
        } catch {}
    }
}
