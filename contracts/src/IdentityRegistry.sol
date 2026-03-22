// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {AccessControl} from "@openzeppelin/contracts/access/AccessControl.sol";

/**
 * @title IdentityRegistry
 * @author POOOL.app
 * @notice Central registry for KYC whitelisted addresses.
 *         All AssetToken clones refer to this single registry to determine
 *         if an address is permitted to hold tokens.
 */
contract IdentityRegistry is AccessControl {
    bytes32 public constant KYC_ADMIN_ROLE = keccak256("KYC_ADMIN_ROLE");

    mapping(address => bool) public isWhitelisted;

    event AddressWhitelisted(address indexed account, bool status);

    error NotAuthorized();
    error ZeroAddress();
    error ArrayLengthMismatch();

    constructor(address initialAdmin) {
        if (initialAdmin == address(0)) revert ZeroAddress();
        
        _grantRole(DEFAULT_ADMIN_ROLE, initialAdmin);
        _grantRole(KYC_ADMIN_ROLE, initialAdmin);

        // Whitelist the admin by default
        isWhitelisted[initialAdmin] = true;
        emit AddressWhitelisted(initialAdmin, true);
    }

    /**
     * @notice Set the whitelist status of a single address.
     * @param account The address to update.
     * @param status True to whitelist, false to remove.
     */
    function setWhitelisted(address account, bool status) external onlyRole(KYC_ADMIN_ROLE) {
        if (account == address(0)) revert ZeroAddress();
        isWhitelisted[account] = status;
        emit AddressWhitelisted(account, status);
    }

    /**
     * @notice Set the whitelist status for multiple addresses in batch.
     * @param accounts Array of addresses.
     * @param statuses Array of boolean statuses.
     */
    function batchSetWhitelisted(address[] calldata accounts, bool[] calldata statuses) external onlyRole(KYC_ADMIN_ROLE) {
        if (accounts.length != statuses.length) revert ArrayLengthMismatch();
        
        for (uint256 i = 0; i < accounts.length; ) {
            if (accounts[i] == address(0)) revert ZeroAddress();
            isWhitelisted[accounts[i]] = statuses[i];
            emit AddressWhitelisted(accounts[i], statuses[i]);
            unchecked { ++i; }
        }
    }

    /**
     * @notice Check if an address is whitelisted.
     * @param account The address to check.
     * @return True if whitelisted, false otherwise.
     */
    function checkWhitelisted(address account) external view returns (bool) {
        return isWhitelisted[account];
    }
}
