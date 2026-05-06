// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Clones} from "@openzeppelin/contracts/proxy/Clones.sol";
import {AccessControl} from "@openzeppelin/contracts/access/AccessControl.sol";

// Interfaces to typecast the initialized clone and call the initialization function
interface IPOOOLAssetToken {
    function initialize(
        address admin,
        address _identityRegistry,
        string calldata assetURI_,
        uint256 initialSupply,
        address mintTo
    ) external;
    
    function mint(address to, uint256 amount) external;
}

/**
 * @title AssetFactory
 * @author POOOL.app
 * @notice Factory contract utilizing EIP-1167 Minimal Proxies (Clones) to deploy 
 *         gas-efficient, isolated smart contracts for each property on the platform.
 */
contract AssetFactory is AccessControl {
    bytes32 public constant DEPLOYER_ROLE = keccak256("DEPLOYER_ROLE");

    /// @notice The address of the master POOOLAssetToken implementation contract
    address public implementationContract;

    /// @notice The central IdentityRegistry that all new clones will point to
    address public identityRegistry;

    /// @notice Array holding all deployed property contract addresses
    address[] public deployedAssets;

    event AssetDeployed(
        address indexed cloneAddress,
        string uri,
        uint256 initialSupply,
        address indexed mintTo
    );

    error ZeroAddress();
    error ZeroSupply();
    error EmptyURI();

    /**
     * @param admin Address to receive roles
     * @param _implementationContract The deployed Implementation logic contract
     * @param _identityRegistry The central KYC IdentityRegistry
     */
    constructor(
        address admin,
        address _implementationContract,
        address _identityRegistry
    ) {
        if (admin == address(0) || _implementationContract == address(0) || _identityRegistry == address(0)) {
            revert ZeroAddress();
        }

        implementationContract = _implementationContract;
        identityRegistry = _identityRegistry;

        _grantRole(DEFAULT_ADMIN_ROLE, admin);
        _grantRole(DEPLOYER_ROLE, admin);
    }

    /**
     * @notice Deploys a new isolated Asset Token for a property via EIP-1167.
     * @param adminForClone The address that will hold admin roles on the new token
     * @param assetURI IPFS URI containing property Metadata (e.g. SPV docs)
     * @param initialSupply Exactly how many fractional shares this property has
     * @param mintTo The initial treasury or investor wallet holding the shares
     * @return cloneAddress The address of the newly deployed Asset Token
     */
    function deployAsset(
        address adminForClone,
        string calldata assetURI,
        uint256 initialSupply,
        address mintTo
    ) external onlyRole(DEPLOYER_ROLE) returns (address cloneAddress) {
        if (adminForClone == address(0) || mintTo == address(0)) revert ZeroAddress();
        if (initialSupply == 0) revert ZeroSupply();
        if (bytes(assetURI).length == 0) revert EmptyURI();

        // 1. Deploy the precise minimal proxy clone
        cloneAddress = Clones.clone(implementationContract);

        // 2. Initialize the clone's storage variables, roles, AND perform initial minting
        IPOOOLAssetToken cloneToken = IPOOOLAssetToken(cloneAddress);
        cloneToken.initialize(adminForClone, identityRegistry, assetURI, initialSupply, mintTo);

        // 3. Keep track of deployed assets
        deployedAssets.push(cloneAddress);

        emit AssetDeployed(cloneAddress, assetURI, initialSupply, mintTo);

        return cloneAddress;
    }

    /**
     * @notice Get all deployed asset contracts.
     */
    function getDeployedAssets() external view returns (address[] memory) {
        return deployedAssets;
    }

    /**
     * @notice Update the implementation contract if we need a v2 logic upgrade
     *         for FUTURE properties (existing clones are immutable).
     */
    function setImplementationContract(address newImplementation) external onlyRole(DEFAULT_ADMIN_ROLE) {
        if (newImplementation == address(0)) revert ZeroAddress();
        implementationContract = newImplementation;
    }

    /**
     * @notice Update the identity registry for FUTURE properties.
     */
    function setIdentityRegistry(address newRegistry) external onlyRole(DEFAULT_ADMIN_ROLE) {
        if (newRegistry == address(0)) revert ZeroAddress();
        identityRegistry = newRegistry;
    }
}
