// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {ERC1155} from "@openzeppelin/contracts/token/ERC1155/ERC1155.sol";
import {ERC1155Supply} from "@openzeppelin/contracts/token/ERC1155/extensions/ERC1155Supply.sol";
import {AccessControl} from "@openzeppelin/contracts/access/AccessControl.sol";
import {Pausable} from "@openzeppelin/contracts/utils/Pausable.sol";
import {IdentityRegistry} from "./IdentityRegistry.sol";

/**
 * @title POOOLAssetToken
 * @author POOOL.app
 * @notice ERC-1155 Implementation Token designed to be cloned via EIP-1167.
 *         Each instance of this contract represents a single real-world property.
 *         The implementation logic is deployed once to save gas.
 */
contract POOOLAssetToken is ERC1155, ERC1155Supply, AccessControl, Pausable {
    bytes32 public constant MINTER_ROLE = keccak256("MINTER_ROLE");
    bytes32 public constant PAUSER_ROLE = keccak256("PAUSER_ROLE");
    bytes32 public constant SETTLEMENT_ROLE = keccak256("SETTLEMENT_ROLE");

    /// @notice The single token ID representing the main asset in this contract
    uint256 public constant ASSET_TOKEN_ID = 1;

    /// @notice Indicates if this clone has been initialized. (Prevents double init)
    bool private _initialized;

    /// @notice The central KYC registry
    IdentityRegistry public identityRegistry;

    /// @notice The IPFS URI containing metadata/docs for this property
    string private _assetURI;

    /// @notice Maximum ownership percentage in basis points (8000 = 80%)
    uint256 public constant MAX_OWNERSHIP_BPS = 8000;
    uint256 public constant MAX_BPS = 10_000;

    event Initialized(address indexed admin, address indexed identityRegistry, string uri);
    event AssetMinted(address indexed mintTo, uint256 amount);
    event BatchSettled(uint256 indexed batchSize, address indexed caller);

    error AlreadyInitialized();
    error NotWhitelisted(address account);
    error MaxOwnershipExceeded(address account, uint256 wouldOwn, uint256 maxAllowed);
    error ArrayLengthMismatch();
    error ZeroAddress();

    /// @notice Pass an empty string to the base ERC1155 constructor. It's safe for Clones.
    constructor() ERC1155("") {}

    /**
     * @notice Initialize the clone with its specific parameters.
     * @param admin The admin address (factory caller)
     * @param _identityRegistry The address of the central IdentityRegistry
     * @param assetURI_ The URI containing the property's metadata
     */
    function initialize(
        address admin,
        address _identityRegistry,
        string calldata assetURI_,
        uint256 initialSupply,
        address mintTo
    ) external {
        if (_initialized) revert AlreadyInitialized();
        if (admin == address(0) || _identityRegistry == address(0) || mintTo == address(0)) revert ZeroAddress();

        _initialized = true;

        identityRegistry = IdentityRegistry(_identityRegistry);
        _assetURI = assetURI_;

        // Setup Roles
        _grantRole(DEFAULT_ADMIN_ROLE, admin);
        _grantRole(MINTER_ROLE, admin);
        _grantRole(PAUSER_ROLE, admin);
        _grantRole(SETTLEMENT_ROLE, admin);

        // Mint initial supply
        if (!identityRegistry.checkWhitelisted(mintTo)) revert NotWhitelisted(mintTo);
        _mint(mintTo, ASSET_TOKEN_ID, initialSupply, "");
        emit AssetMinted(mintTo, initialSupply);

        emit Initialized(admin, _identityRegistry, assetURI_);
    }

    /**
     * @notice Returns the URI for the asset token.
     *         Overrides standard ERC1155 uri function.
     */
    function uri(uint256 tokenId) public view virtual override returns (string memory) {
        // We only use tokenId = 1 for the main asset
        require(tokenId == ASSET_TOKEN_ID, "Token ID not 1");
        return _assetURI;
    }

    /**
     * @notice Set or update the URI.
     */
    function setURI(string calldata newuri) external onlyRole(DEFAULT_ADMIN_ROLE) {
        _assetURI = newuri;
    }

    /**
     * @notice Pause token transfers in emergencies.
     */
    function pause() external onlyRole(PAUSER_ROLE) {
        _pause();
    }

    /**
     * @notice Unpause token transfers.
     */
    function unpause() external onlyRole(PAUSER_ROLE) {
        _unpause();
    }

    /**
     * @notice Initial primary offering mint. Once minted, supply is fixed unless further rounds.
     * @param to Address to mint shares to
     * @param amount The number of fractional shares to mint
     */
    function mint(address to, uint256 amount) external onlyRole(MINTER_ROLE) {
        if (!identityRegistry.checkWhitelisted(to)) revert NotWhitelisted(to);
        
        _mint(to, ASSET_TOKEN_ID, amount, "");
        emit AssetMinted(to, amount);
    }

    /**
     * @notice Batch settlement function for end-of-day netting by the backend worker.
     * @param froms Senders array
     * @param tos Recipients array
     * @param amounts Token amounts
     */
    function settleBatch(
        address[] calldata froms,
        address[] calldata tos,
        uint256[] calldata amounts
    ) external onlyRole(SETTLEMENT_ROLE) {
        if (froms.length != tos.length || tos.length != amounts.length) revert ArrayLengthMismatch();

        for (uint256 i = 0; i < froms.length; ) {
            // _update bypasses SafeTransferFrom approval checks
            uint256[] memory ids = new uint256[](1);
            ids[0] = ASSET_TOKEN_ID;
            uint256[] memory vals = new uint256[](1);
            vals[0] = amounts[i];
            
            _update(froms[i], tos[i], ids, vals);
            unchecked { ++i; }
        }
        
        emit BatchSettled(froms.length, msg.sender);
    }

    /**
     * @notice Hook that is called before any token transfer. This includes minting and burning.
     */
    function _update(
        address from,
        address to,
        uint256[] memory ids,
        uint256[] memory values
    ) internal virtual override(ERC1155, ERC1155Supply) whenNotPaused {
        super._update(from, to, ids, values);

        for (uint256 i = 0; i < ids.length; ++i) {
            uint256 tokenId = ids[i];

            // Burning is always allowed, bypass other checks
            if (to == address(0)) continue;

            // Enforce KYC Status
            if (!identityRegistry.checkWhitelisted(to)) revert NotWhitelisted(to);

            // Calculate resulting token ownership (balanceOf already includes the transferred value due to super._update)
            uint256 userNewBalance = balanceOf(to, tokenId);

            // Maximum ownership enforcement (e.g. 80%)
            // We use totalSupply(tokenId) which after super._update already contains the newly minted amount
            uint256 currentSupply = totalSupply(tokenId);
            
            // Skip check if total supply is 0 or if it's a mint operation
            // (e.g. treasury minting initial supply)
            if (currentSupply > 0 && from != address(0)) {
                uint256 maxAllowed = (currentSupply * MAX_OWNERSHIP_BPS) / MAX_BPS;

                if (userNewBalance > maxAllowed) {
                    revert MaxOwnershipExceeded(to, userNewBalance, maxAllowed);
                }
            }
        }
    }

    /**
     * @notice Required override for multiple inheritance resolving
     */
    function supportsInterface(bytes4 interfaceId)
        public
        view
        override(ERC1155, AccessControl)
        returns (bool)
    {
        return super.supportsInterface(interfaceId);
    }
}
