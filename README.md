---

ULIQ Ritual Contracts

Cross‑chain ULIQ contracts deployed on four EVM networks.
Each contract emits a deterministic chainProof when a user completes a ritual step.
These proofs are collected by an off‑chain Aggregator, assembled into a ritualHash, and submitted to Solana to mint a Legendary NFT that funds a real‑world well.

This repo contains the EVM‑side anchor of the ritual.

---

✨ Overview

The ULIQ ritual is a four‑chain progression:

1. User completes a ritual step on Chain A → receives chainProofA
2. User completes a ritual step on Chain B → receives chainProofB
3. User completes a ritual step on Chain C → receives chainProofC
4. User completes a ritual step on Chain D → receives chainProofD


Each chain runs its own instance of the ULIQClaim contract.

An off‑chain Aggregator listens to all four chains, collects the proofs, and builds:

ritualHash = keccak256(
    user,
    chainProofA,
    chainProofB,
    chainProofC,
    chainProofD,
    version,
    timestamp
)


The Aggregator then submits this hash to the LegendaryMint program on Solana, which mints a Legendary NFT and triggers the real‑world impact workflow.

---

🧱 Contract: ULIQClaim

Each EVM chain hosts one instance of this contract.

Features

• Deterministic per‑chain chainProof
• Replay protection (hasClaimed)
• Event‑driven architecture for Aggregator listeners
• Minimal, gas‑efficient, and chain‑agnostic
• Supports versioned or branched rituals via stepId


Key Event

event ULIQClaimed(
    address indexed user,
    uint256 indexed chainId,
    bytes32 chainProof,
    uint256 stepId
);


The Aggregator listens to this event across all four chains.

---

📦 Repository Structure

uliq-ritual/
  ├─ contracts/
  │   ├─ ULIQClaim.sol
  │   └─ interfaces/
  │       └─ IULIQClaim.sol
  ├─ scripts/
  │   └─ deploy.ts
  ├─ test/
  │   └─ uliq.test.ts
  ├─ hardhat.config.ts
  ├─ package.json
  ├─ .gitignore
  └─ README.md


---

🚀 Getting Started

Install dependencies

npm install


Compile contracts

npx hardhat compile


Run tests

npx hardhat test


Deploy to a network

npx hardhat run scripts/deploy.ts --network <networkName>


---

🔗 How the Ritual Works (Developer Flow)

1. User calls `claim(stepId)`

On each chain, the user completes the ritual step:

ULIQClaim.claim(stepId)


2. Contract emits `ULIQClaimed`

The event includes:

• user
• chainId
• chainProof
• stepId


3. Aggregator listens to all chains

It stores:

ritualState[user][chainId] = chainProof


4. When all four proofs exist

The Aggregator builds the ritualHash.

5. Aggregator submits to Solana

The Solana program:

• Verifies the ritual
• Mints the Legendary NFT
• Emits LegendaryMinted


6. Real‑world impact triggers

A well is funded and tracked.

---

🛡️ Security Model

• Each user can claim once per chain
• Proofs are bound to:• user
• CHAIN_ID
• stepId
• address(this)
• block.chainid (extra safety)

• No admin privileges required for claiming
• No external dependencies
• No upgradeability (simple, immutable contracts)


---

🌍 Multi‑Chain Deployment

Deploy one instance of ULIQClaim on each EVM chain:

Chain	Contract	Purpose	
Chain A	ULIQClaim	Ritual Step 1	
Chain B	ULIQClaim	Ritual Step 2	
Chain C	ULIQClaim	Ritual Step 3	
Chain D	ULIQClaim	Ritual Step 4	


Each instance uses a different CHAIN_ID passed to the constructor.

---

🧪 Local Testing

You can simulate the full ritual locally:

npx hardhat node
npx hardhat run scripts/deploy.ts --network localhost


Then call:

npx hardhat console --network localhost


And interact with the contract:

await uliq.claim(1)
await uliq.getChainProof(user.address)


---

📜 License

This project is licensed under the MIT License.
You are free to use, modify, fork, and integrate these contracts in any metaverse or application.

---

🤝 Contributing

Pull requests, issues, and extensions are welcome.
This protocol is designed to be a public good powering cross‑chain rituals and real‑world impact
