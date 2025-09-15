To deploy the contracts in Solana's localnet, run `solana-test-validator` and then
`anchor build --skip-lint` and `anchor deploy`.

The scripts can be simply ran as: cargo run --bin `script name`

The approve message script makes the necessary initializations of the contract, so run that once before the other ones.

Note : The contracts are a very simple dummy version, trying to emit similar events to the actual ones in the devnet. Once the actual contracts have been deployed, it is recommended to switch over to using them. 