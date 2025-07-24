# Solana Multisig Wallet

A secure, on-chain multisig wallet implementation built with Anchor Framework for the Solana blockchain. This program enables multiple parties to collectively control a wallet, requiring a minimum number of signatures (threshold) to execute transactions.

## 🚀 Features

- **Multi-signature Security**: Require multiple approvals before executing transactions
- **Flexible Threshold**: Configure M-of-N signature requirements (e.g., 2-of-3, 3-of-5)
- **Transaction Proposals**: Any owner can propose transactions for group approval
- **Cross-Program Invocation**: Execute transactions to any Solana program
- **Nonce Account Support**: Optional integration with system nonce accounts for replay protection
- **Event Emission**: Comprehensive logging for transaction lifecycle
- **Duplicate Prevention**: Built-in nonce tracking to prevent transaction replay
- **Gas Optimization**: Efficient account space allocation and data management

## 📋 Program Overview

The multisig wallet consists of three main operations:

1. **Initialize**: Create a new multisig wallet with owners and threshold
2. **Create Transaction**: Propose a new transaction for approval
3. **Approve Transaction**: Owners vote to approve proposed transactions
4. **Execute Transaction**: Execute approved transactions via CPI

## 🏗️ Architecture

### Core Accounts

- **Multisig**: Main wallet account storing owners, threshold, and metadata
- **Transaction**: Individual transaction proposals with approval tracking

### Security Features

- Owner validation and duplicate prevention
- Threshold enforcement before execution
- Nonce replay protection
- Authority validation for nonce accounts

## 🛠️ Installation & Setup

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [Solana CLI](https://docs.solana.com/cli/install-solana-cli-tools) (v1.14+)
- [Anchor Framework](https://www.anchor-lang.com/docs/installation) (v0.28+)

### Clone & Build

```bash
git clone https://github.com/yourusername/multisig-wallet.git
cd multisig-wallet

# Install dependencies
npm install

# Build the program
anchor build

# Run tests
anchor test

# Deploy to devnet
anchor deploy --provider.cluster devnet
```

## 📖 Usage Examples

### 1. Initialize a Multisig Wallet

```javascript
const multisigId = new BN(1);
const owners = [owner1.publicKey, owner2.publicKey, owner3.publicKey];
const threshold = 2; // 2-of-3 signatures required

await program.methods
  .initialize(multisigId, owners, threshold)
  .accounts({
    multisig: multisigPda,
    creator: owner1.publicKey,
    systemProgram: SystemProgram.programId,
  })
  .signers([owner1])
  .rpc();
```

### 2. Create a Transaction Proposal

```javascript
const nonce = new BN(Date.now());
const instruction = SystemProgram.transfer({
  fromPubkey: multisigPda,
  toPubkey: recipient.publicKey,
  lamports: LAMPORTS_PER_SOL,
});

await program.methods
  .createTransaction(
    multisigId,
    nonce,
    instruction.programId,
    instruction.keys.map(key => ({
      pubkey: key.pubkey,
      isSigner: key.isSigner,
      isWritable: key.isWritable,
    })),
    instruction.data
  )
  .accounts({
    proposer: owner1.publicKey,
    multisig: multisigPda,
    transaction: transactionPda,
    systemProgram: SystemProgram.programId,
  })
  .signers([owner1])
  .rpc();
```

### 3. Approve a Transaction

```javascript
await program.methods
  .approveTransaction(multisigId, nonce)
  .accounts({
    owner: owner2.publicKey,
    multisig: multisigPda,
    transaction: transactionPda,
  })
  .signers([owner2])
  .rpc();
```

### 4. Execute Approved Transaction

```javascript
await program.methods
  .executeTransaction(multisigId, nonce)
  .accounts({
    executor: owner1.publicKey,
    multisig: multisigPda,
    transaction: transactionPda,
  })
  .remainingAccounts([
    { pubkey: multisigPda, isSigner: false, isWritable: true },
    { pubkey: recipient.publicKey, isSigner: false, isWritable: true },
    { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
  ])
  .signers([owner1])
  .rpc();
```

## 🔧 Configuration

### Constants (Configurable in `lib.rs`)

```rust
const MAX_OWNERS: usize = 10;                    // Maximum number of owners
const MAX_STORED_NONCES: usize = 100;           // Nonce history size
const MAX_INSTRUCTION_ACCOUNTS: usize = 10;     // Max accounts per transaction
const MAX_INSTRUCTION_DATA_SIZE: usize = 1024;  // Max instruction data size
```

## 📊 Events

The program emits the following events for monitoring:

```rust
// Transaction created
TransactionCreated {
    multisig: Pubkey,
    transaction: Pubkey,
    proposer: Pubkey,
    nonce: u64,
}

// Transaction approved
TransactionApproved {
    transaction: Pubkey,
    approver: Pubkey,
    approvals_count: u8,
    threshold: u8,
}

// Transaction executed
TransactionExecuted {
    transaction: Pubkey,
    executor: Pubkey,
}
```

## ⚠️ Error Codes

| Code | Error | Description |
|------|-------|-------------|
| 6000 | InvalidThreshold | Threshold exceeds number of owners |
| 6001 | DuplicateOwners | Duplicate addresses in owners list |
| 6002 | NoOwners | Empty owners list provided |
| 6003 | NotAnOwner | Signer is not a wallet owner |
| 6004 | AlreadyApproved | Owner already approved this transaction |
| 6005 | NonceAlreadyUsed | Transaction nonce already used |
| 6006 | AlreadyExecuted | Transaction already executed |
| 6007 | NotEnoughApprovals | Insufficient approvals for execution |
| 6008 | TooManyAccounts | Too many accounts in transaction |
| 6009 | InstructionDataTooLarge | Instruction data exceeds size limit |

## 🧪 Testing

```bash
# Run all tests
anchor test

# Run specific test file
anchor test --skip-local-validator tests/multisig.ts

# Test with different clusters
anchor test --provider.cluster devnet
```

## 🚧 Future Enhancements

- [ ] **Owner Management**: Add/remove owners, change threshold
- [ ] **Transaction Management**: Cancel transactions, set expiration
- [ ] **Batch Operations**: Execute multiple transactions atomically  
- [ ] **Recovery Mechanisms**: Emergency recovery procedures
- [ ] **Oracle Integration**: External data source integration
- [ ] **Scheduled Transactions**: Time-based transaction execution
- [ ] **Gas Optimization**: Further reduce transaction costs
- [ ] **Mobile SDK**: React Native integration

## 🔐 Security Considerations

- **Audit Status**: ⚠️ Not audited - use at your own risk
- **Testnet Only**: Recommended for testnet use until audited
- **Owner Key Security**: Secure storage of owner private keys essential
- **Threshold Selection**: Choose appropriate M-of-N ratios for your use case

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Development Guidelines

- Follow Rust and Anchor best practices
- Add comprehensive tests for new features
- Update documentation for API changes
- Ensure all tests pass before submitting PR

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [Anchor Framework](https://github.com/coral-xyz/anchor) - Solana development framework
- [Solana Labs](https://github.com/solana-labs/solana) - Blockchain infrastructure
- Community contributors and testers

## 📞 Support

- **Issues**: [GitHub Issues](https://github.com/yourusername/multisig-wallet/issues)
- **Discussions**: [GitHub Discussions](https://github.com/yourusername/multisig-wallet/discussions)
- **Documentation**: [Wiki](https://github.com/yourusername/multisig-wallet/wiki)

---

**⚠️ Disclaimer**: This software is provided "as is" without warranties. Use at your own risk. Always test thoroughly on devnet before mainnet deployment.