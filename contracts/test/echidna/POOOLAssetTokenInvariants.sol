// SPDX-License-Identifier: MIT
pragma solidity 0.8.24;

import {AssetFactory} from "../../src/AssetFactory.sol";
import {POOOLAssetToken} from "../../src/POOOLAssetToken.sol";
import {IdentityRegistry} from "../../src/IdentityRegistry.sol";

/// @title POOOL — Echidna property tests
/// @notice Property-based fuzzer (coverage-guided, multi-call sequences)
///         that goes deeper than Foundry's stateless fuzz. Each
///         `echidna_*` returns true when the invariant holds — Echidna
///         tries millions of call sequences to disprove it.
contract POOOLAssetTokenInvariants {
    AssetFactory public factory;
    POOOLAssetToken public implementation;
    IdentityRegistry public registry;
    POOOLAssetToken public clone;

    uint256 public constant SUPPLY = 1_000_000;
    uint256 public constant TOKEN_ID = 1;
    uint256 public constant MAX_BPS = 10_000;
    uint256 public constant MAX_OWNERSHIP_BPS = 8_000;

    address public constant ADMIN = address(0xa11ce);
    address public constant USER1 = address(0xb0b);
    address public constant USER2 = address(0xca7);

    constructor() {
        registry = new IdentityRegistry(ADMIN);
        implementation = new POOOLAssetToken();
        factory = new AssetFactory(ADMIN, address(implementation), address(registry));

        // Use vm-like prank by calling as ADMIN via low-level — Echidna
        // invokes constructor as the deployer; we mirror admin actions
        // via direct calls below in handler functions.
        // For initialize, ADMIN must be the caller because Factory has
        // DEPLOYER_ROLE on ADMIN. Echidna handlers call factory directly.
    }

    // ─── Handler functions Echidna can call ─────────────────────

    /// Whitelist a fixed test user (idempotent).
    function whitelistUser1() external {
        if (msg.sender == ADMIN) registry.setWhitelisted(USER1, true);
    }

    function whitelistUser2() external {
        if (msg.sender == ADMIN) registry.setWhitelisted(USER2, true);
    }

    function deployClone() external {
        if (address(clone) != address(0)) return;
        if (msg.sender != ADMIN) return;
        address c = factory.deployAsset(ADMIN, "ipfs://t", SUPPLY, ADMIN);
        clone = POOOLAssetToken(c);
    }

    function transfer(uint256 amount) external {
        if (address(clone) == address(0)) return;
        amount = amount % (SUPPLY + 1);
        if (amount == 0) return;
        try clone.safeTransferFrom(msg.sender, USER1, TOKEN_ID, amount, "") {} catch {}
    }

    function pauseClone() external {
        if (address(clone) == address(0)) return;
        if (msg.sender == ADMIN) clone.pause();
    }

    function unpauseClone() external {
        if (address(clone) == address(0)) return;
        if (msg.sender == ADMIN) clone.unpause();
    }

    // ─── Invariants ──────────────────────────────────────────────

    /// I1: Total supply only changes via mint/burn — never via transfer.
    /// Since this contract never burns, supply is constant after deploy.
    function echidna_supplyConstant() public view returns (bool) {
        if (address(clone) == address(0)) return true;
        return clone.totalSupply(TOKEN_ID) == SUPPLY;
    }

    /// I2: No single non-treasury holder ever exceeds 80% of supply.
    function echidna_maxOwnershipRespected() public view returns (bool) {
        if (address(clone) == address(0)) return true;
        uint256 maxAllowed = (clone.totalSupply(TOKEN_ID) * MAX_OWNERSHIP_BPS) / MAX_BPS;
        // Sample the test users; mint() to ADMIN bypasses the cap (from==0)
        // and admin holds initial supply, so we check non-ADMIN holders.
        return clone.balanceOf(USER1, TOKEN_ID) <= maxAllowed
            && clone.balanceOf(USER2, TOKEN_ID) <= maxAllowed;
    }

    /// I3: Sum of tracked balances never exceeds total supply (no
    /// minting via transfers).
    function echidna_balanceSumLEQSupply() public view returns (bool) {
        if (address(clone) == address(0)) return true;
        uint256 sum = clone.balanceOf(ADMIN, TOKEN_ID)
                    + clone.balanceOf(USER1, TOKEN_ID)
                    + clone.balanceOf(USER2, TOKEN_ID);
        return sum <= clone.totalSupply(TOKEN_ID);
    }

    /// I4: Non-whitelisted addresses never hold tokens.
    function echidna_nonWhitelistedHasZero() public view returns (bool) {
        if (address(clone) == address(0)) return true;
        // Pick an address we never whitelist
        address ghost = address(0xdead);
        return clone.balanceOf(ghost, TOKEN_ID) == 0;
    }

    /// I5: ADMIN never loses DEFAULT_ADMIN_ROLE.
    function echidna_adminRoleStable() public view returns (bool) {
        if (address(clone) == address(0)) return true;
        return clone.hasRole(0x00, ADMIN); // DEFAULT_ADMIN_ROLE = bytes32(0)
    }
}
