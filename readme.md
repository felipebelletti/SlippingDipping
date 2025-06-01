## Archived
I'm disclosing this project since I'm no longer actively working on it.

This project contains a full Proof of Concept (PoC) for exploiting ERC20 tokens on Ethereum. Specifically, it targets token contracts that hold a portion of their own tokens to implement a tax mechanism through token sales, which generates profit for the token developer.

Note: Sensitive information has been redacted and is marked with \*\*\*REMOVED\*\*\* throughout the codebase.

## Libraries
- [Artemis](https://github.com/paradigmxyz/artemis)
- [Revm](https://github.com/bluealloy/revm)
- [Amms-rs](https://github.com/darkforestry/amms-rs)

### Test
`forge test --fork-url http://localhost:8545 -vvvv --match-test testExploitM1M6`

`anvil -f https://mainnet.infura.io/v3/e4a9193a1f35493786c001c3573f7905 --fork-block-number 20757391`