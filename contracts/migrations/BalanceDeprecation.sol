// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

interface IVault {
    function balance(address user) external view returns (uint256);
    function shares(address user) external view returns (uint256);
    function sharePrice() external view returns (uint256);
}

contract BalanceDeprecation {
    IVault public vault;
    uint256 constant PRECISION = 1e18;
    
    constructor(address _vault) {
        vault = IVault(_vault);
    }
}
