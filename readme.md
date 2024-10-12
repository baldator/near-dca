# DCA Smart Contract for NEAR Network

This repository provides a Rust-based smart contract for implementing a Dollar Cost Averaging (DCA) strategy on the NEAR network. 

### Features

- **Multi-user:** Allows multiple users to enroll in the DCA program.
- **Batch Swapping:** Minimizes swapping fees by grouping user swaps into a single transaction when reaching a specific threshold.
- **Customizable:** Users can define their own amount per swap and swap interval.

### Getting Started

1. **Prerequisites:**
    - Rust compiler (`rustc`) installed.
    - NEAR development environment set up ([invalid URL removed]).

2. **Clone the repository:**

```bash
git clone [https://github.com/baladtor/near-dca.git](https://github.com//baladtor/near-dca.git)
```

3. **Build the contract:**

```bash
cd dca-near-contract
cargo build --release
```

4. **Deploy the contract:**

Use the near-cli tool to deploy the contract to your NEAR account. Refer to the NEAR documentation for detailed deployment instructions: [invalid URL removed]

**Note:** This README assumes basic familiarity with Rust and NEAR development.

### Using the Contract
1. **Register as a user:**

Call the register_user method with your desired amount_per_swap (in NEAR tokens) and swap_interval (in block timestamps). The swap_interval determines how often the contract automatically performs a swap for you.

2. **Trigger a swap:**
Call the swap method to initiate a swap. This function checks if the swap interval has passed and performs a single swap or a batch swap if the user threshold is met.

### Security Considerations

This is a basic implementation and may require additional security measures in production environments.
Ensure you trust the chosen DEX aggregator for swapping tokens.

### Further Development

- Implement error handling for contract methods.
- Enhance the batch_swap function with a chosen DEX aggregator library for NEAR.
- Consider adding features like stop-loss or profit-taking mechanisms.

### Contributing

We welcome contributions to this project. Please open a pull request to share your improvements.