const { expect } = require("chai");
const { ethers } = require("hardhat");

describe("ULIQLegendaryNFT Signature Mint", function () {
  let nft, owner, signer, user;

  beforeEach(async function () {
    // 1. Setup accounts
    [owner, signer, user] = await ethers.getSigners();

    // 2. Deploy contract with 'signer.address' as the trusted source
    const ULIQ = await ethers.getContractFactory("ULIQLegendaryNFT");
    nft = await ULIQ.deploy(signer.address, "https://api.uliq.com/metadata/");
  });

  it("Should allow minting with a valid signature", async function () {
    const chainId = (await ethers.provider.getNetwork()).chainId;
    const contractAddress = await nft.getAddress();

    // 3. Backend Logic: Create the message hash
    const messageHash = ethers.solidityPackedKeccak256(
      ["address", "uint256", "address"],
      [user.address, chainId, contractAddress]
    );

    // 4. Backend Logic: Sign the hash using the 'signer' private key
    // signMessage automatically adds the "\x19Ethereum Signed Message:\n32" prefix
    const signature = await signer.signMessage(ethers.getBytes(messageHash));

    // 5. Frontend Logic: Call the contract with the signature
    await expect(nft.connect(user).mintLegendary(signature))
      .to.emit(nft, "Transfer") // Standard ERC721 mint event
      .withArgs(ethers.ZeroAddress, user.address, 0);

    expect(await nft.hasMinted(user.address)).to.equal(true);
  });

  it("Should fail if the signature is signed by an unauthorized account", async function () {
    const chainId = (await ethers.provider.getNetwork()).chainId;
    const contractAddress = await nft.getAddress();

    // A malicious actor signs their own message
    const messageHash = ethers.solidityPackedKeccak256(
      ["address", "uint256", "address"],
      [user.address, chainId, contractAddress]
    );
    const fakeSignature = await user.signMessage(ethers.getBytes(messageHash));

    await expect(nft.connect(user).mintLegendary(fakeSignature))
      .to.be.revertedWithCustomError(nft, "InvalidSignature");
  });

  it("Should prevent double minting", async function () {
    const chainId = (await ethers.provider.getNetwork()).chainId;
    const contractAddress = await nft.getAddress();

    const messageHash = ethers.solidityPackedKeccak256(
      ["address", "uint256", "address"],
      [user.address, chainId, contractAddress]
    );
    const signature = await signer.signMessage(ethers.getBytes(messageHash));

    // First mint succeeds
    await nft.connect(user).mintLegendary(signature);

    // Second mint fails
    await expect(nft.connect(user).mintLegendary(signature))
      .to.be.revertedWithCustomError(nft, "AlreadyMinted");
  });
});
