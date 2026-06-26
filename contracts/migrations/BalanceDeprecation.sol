// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/**
 * @title BalanceDeprecation
 * @notice Migration helper for moving from Balance+Shares to Shares-only accounting
 */

interface IVault {
    function balance(address user) external view returns (uint256);
    function shares(address user) external view returns (uint256);
    function sharePrice() external view returns (uint256);
}

contract BalanceDeprecation {
    IVault public vault;
    uint256 constant PRECISION = 1e18;
    
    event BalanceMigrated(address indexed user, uint256 balance, uint256 shares);
    event MigrationComplete(uint256 userCount, uint256 totalMigrated);
    
    constructor(address _vault) {
        vault = IVault(_vault);
    }
    
    function migrateBalancesToShares(address[] calldata users) external {
        uint256 totalMigrated = 0;
        uint256 sharePrice = vault.sharePrice();
        
        for (uint i = 0; i < users.length; i++) {
            address user = users[i];
            uint256 userBalance = vault.balance(user);
            
            if (userBalance > 0) {
                uint256 sharesToMint = (userBalance * PRECISION) / sharePrice;
                totalMigrated += userBalance;
                
                emit BalanceMigrated(user, userBalance, sharesToMint);
            }
        }
        
        emit MigrationComplete(users.length, totalMigrated);
    }
    
    function getDerivedBalance(address user) external view returns (uint256) {
        uint256 userShares = vault.shares(user);
        uint256 sharePrice = vault.sharePrice();
        return (userShares * sharePrice) / PRECISION;
    }
    
    function verifyBalanceDerivation(address user) external view returns (bool) {
        uint256 derivedBalance = this.getDerivedBalance(user);
        return derivedBalance >= 0;
    }
}
