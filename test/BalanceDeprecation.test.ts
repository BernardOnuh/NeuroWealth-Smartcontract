import { expect } from "chai";
import { ethers } from "hardhat";

describe("Balance Deprecation - Shares-Only Accounting", function () {
  let vault: any;
  let migration: any;
  let user1: any;
  let user2: any;
  
  const PRECISION = ethers.BigNumber.from(10).pow(18);
  const SHARE_PRICE = PRECISION;

  beforeEach(async () => {
    [user1, user2] = await ethers.getSigners();
    
    const VaultFactory = await ethers.getContractFactory("MockVault");
    vault = await VaultFactory.deploy();
    
    const MigrationFactory = await ethers.getContractFactory("BalanceDeprecation");
    migration = await MigrationFactory.deploy(vault.address);
  });

  describe("Migration Correctness", function () {
    it("should derive balance from shares correctly", async () => {
      const depositAmount = ethers.utils.parseEther("100");
      await vault.setShares(user1.address, depositAmount);
      
      const derived = await migration.getDerivedBalance(user1.address);
      expect(derived).to.equal(depositAmount);
    });

    it("get_balance() should match share-derived assets", async () => {
      const amount = ethers.utils.parseEther("10");
      await vault.deposit(user1.address, amount);
      
      const shares = await vault.shares(user1.address);
      const balance = await vault.getBalance(user1.address);
      const derived = (shares.mul(SHARE_PRICE)).div(PRECISION);
      
      expect(balance).to.equal(derived);
    });
  });

  describe("Path Updates", function () {
    it("deposit should mint correct shares", async () => {
      const amount = ethers.utils.parseEther("10");
      await vault.deposit(user1.address, amount);
      
      const shares = await vault.shares(user1.address);
      expect(shares).to.equal(amount.mul(PRECISION).div(SHARE_PRICE));
    });

    it("withdraw should burn correct shares", async () => {
      const depositAmount = ethers.utils.parseEther("10");
      const withdrawAmount = ethers.utils.parseEther("5");
      
      await vault.deposit(user1.address, depositAmount);
      const sharesBefore = await vault.shares(user1.address);
      
      await vault.withdraw(user1.address, withdrawAmount);
      const sharesAfter = await vault.shares(user1.address);
      
      expect(sharesBefore.sub(sharesAfter)).to.equal(
        withdrawAmount.mul(PRECISION).div(SHARE_PRICE)
      );
    });
  });

  describe("Yield Accrual", function () {
    it("shares should not change on yield", async () => {
      const amount = ethers.utils.parseEther("10");
      await vault.deposit(user1.address, amount);
      
      const sharesBefore = await vault.shares(user1.address);
      await vault.accrueYield(ethers.utils.parseEther("2"));
      
      const sharesAfter = await vault.shares(user1.address);
      expect(sharesAfter).to.equal(sharesBefore);
    });

    it("balance should increase with yield", async () => {
      const amount = ethers.utils.parseEther("10");
      await vault.deposit(user1.address, amount);
      
      const balanceBefore = await vault.getBalance(user1.address);
      await vault.accrueYield(ethers.utils.parseEther("2"));
      
      const balanceAfter = await vault.getBalance(user1.address);
      expect(balanceAfter).to.be.gt(balanceBefore);
    });
  });
});
