use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use anyhow::anyhow;

declare_id!("7RdSDLUUy37Wqc6s9ebgo52AwhGiw4XbJWZJgidQ1fJc");

#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NativeGasPaidForContractCallEvent {
    pub config_pda: Pubkey,
    pub destination_chain: String,
    pub destination_address: String,
    pub payload_hash: [u8; 32],
    pub refund_address: Pubkey,
    pub gas_fee_amount: u64,
}

#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CallContractEvent {
    pub sender_key: Pubkey,
    pub payload_hash: [u8; 32],
    pub destination_chain: String,
    pub destination_contract_address: String,
    pub payload: Vec<u8>,
}

#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NativeGasRefundedEvent {
    /// Solana transaction signature
    pub tx_hash: [u8; 64],
    /// The Gas service config PDA
    pub config_pda: Pubkey,
    /// The log index in format "x.y"
    pub log_index: String,
    /// The receiver of the refund
    pub receiver: Pubkey,
    /// amount of SOL
    pub fees: u64,
}

#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NativeGasAddedEvent {
    /// The Gas service config PDA
    pub config_pda: Pubkey,
    /// Solana transaction signature
    pub tx_hash: [u8; 64],
    /// The log index in format "x.y"
    pub log_index: String,
    /// The receiver of the refund
    pub refund_address: Pubkey,
    /// amount of SOL added
    pub gas_fee_amount: u64,
}

#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MessageApprovedEvent {
    pub command_id: [u8; 32],
    pub destination_address: Pubkey,
    pub payload_hash: [u8; 32],
    pub source_chain: String,
    pub message_id: String,
    pub source_address: String,
    pub destination_chain: String,
}

#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MessageExecuted {
    pub command_id: [u8; 32],
    pub destination_address: Pubkey,
    pub payload_hash: [u8; 32],
    pub source_chain: String,
    pub cc_id: String,
    pub source_address: String,
    pub destination_chain: String,
}

#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct InterchainTransfer {
    pub token_id: [u8; 32],
    pub source_address: Pubkey,
    pub source_token_account: Pubkey,
    pub destination_chain: String,
    pub destination_address: Vec<u8>,
    pub amount: u64,
    pub data_hash: [u8; 32],
}

#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LinkTokenStarted {
    pub token_id: [u8; 32],
    pub destination_chain: String,
    pub source_token_address: Pubkey,
    pub destination_token_address: Vec<u8>,
    pub token_manager_type: u8,
    pub params: Vec<u8>,
}

#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct InterchainTokenDeploymentStarted {
    pub token_id: [u8; 32],
    pub token_name: String,
    pub token_symbol: String,
    pub token_decimals: u8,
    pub minter: Vec<u8>,
    pub destination_chain: String,
}

#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TokenMetadataRegistered {
    pub token_address: Pubkey,
    pub decimals: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub struct U256(pub [u8; 32]);

#[event]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifierSetRotatedEvent {
    /// The epoch number as a 256-bit integer in little-endian format
    pub epoch: U256,
    /// Hash of the new verifier set
    pub verifier_set_hash: [u8; 32],
}

#[program]
pub mod program_tester {
    use std::str::FromStr;

    use super::*;

    pub fn pay_native_for_contract_call(
        ctx: Context<PayNativeForContractCall>,
        destination_chain: String,
        destination_address: String,
        payload_hash: [u8; 32],
        refund_address: Pubkey,
        gas_fee_amount: u64,
    ) -> Result<()> {
        let config_pda_key = ctx.accounts.config_pda.key();

        anchor_lang::prelude::emit_cpi!(NativeGasPaidForContractCallEvent {
            config_pda: config_pda_key,
            destination_chain,
            destination_address,
            payload_hash,
            refund_address,
            gas_fee_amount,
        });

        Ok(())
    }
    pub fn refund_native_fees(
        ctx: Context<RefundNativeFees>,
        tx_hash: [u8; 64],
        log_index: String,
        fees: u64,
    ) -> Result<()> {
        anchor_lang::prelude::emit_cpi!(NativeGasRefundedEvent {
            tx_hash,
            config_pda: ctx.accounts.config_pda.key(),
            log_index,
            receiver: ctx.accounts.receiver.key(),
            fees,
        });

        Ok(())
    }

    pub fn add_native_gas(
        ctx: Context<AddNativeGas>,
        tx_hash: [u8; 64],
        log_index: String,
        gas_fee_amount: u64,
        refund_address: Pubkey,
    ) -> Result<()> {
        // Simply emit the event without any on-chain logic (mocked version)
        anchor_lang::prelude::emit_cpi!(NativeGasAddedEvent {
            config_pda: ctx.accounts.config_pda.key(),
            tx_hash,
            log_index,
            refund_address,
            gas_fee_amount,
        });

        Ok(())
    }

    pub fn call_contract(
        ctx: Context<CallContract>,
        destination_chain: String,
        destination_contract_address: String,
        payload_hash: [u8; 32],
        payload: Vec<u8>,
    ) -> Result<()> {
        anchor_lang::prelude::emit_cpi!(CallContractEvent {
            sender_key: ctx.accounts.calling_program.key(),
            destination_chain,
            destination_contract_address,
            payload_hash,
            payload,
        });
        Ok(())
    }

    pub fn approve_message(
        ctx: Context<ApproveMessage>,
        message: MerkleisedMessage,
        payload_merkle_root: [u8; 32],
    ) -> Result<()> {
        let cc_id = &message.leaf.message.cc_id;
        let destination_address =
            Pubkey::from_str(&message.leaf.message.destination_address).unwrap();

        // Initialize the incoming message account
        ctx.accounts
            .incoming_message_pda
            .set_inner(IncomingMessage {
                bump: ctx.bumps.incoming_message_pda,
                signing_pda_bump: 0, // dummy value for now
                status: MessageStatus::approved(),
                message_hash: message.leaf.message.hash(),
                payload_hash: message.leaf.message.payload_hash,
            });

        anchor_lang::prelude::emit_cpi!(MessageApprovedEvent {
            command_id: message.leaf.message.command_id(),
            destination_address,
            payload_hash: message.leaf.message.payload_hash,
            source_chain: cc_id.chain.clone(),
            message_id: cc_id.id.clone(),
            source_address: message.leaf.message.source_address.clone(),
            destination_chain: message.leaf.message.destination_chain.clone(),
        });
        Ok(())
    }

    pub fn execute_message(
        ctx: Context<ExecuteMessage>,
        command_id: [u8; 32],
        source_chain: String,
        cc_id: String,
        source_address: String,
        destination_chain: String,
        destination_address: String,
        payload_hash: [u8; 32],
    ) -> Result<()> {
        let destination_pubkey = Pubkey::from_str(&destination_address).unwrap();

        // Simply emit the event without any on-chain logic checks
        anchor_lang::prelude::emit_cpi!(MessageExecuted {
            command_id,
            destination_address: destination_pubkey,
            payload_hash,
            source_chain,
            cc_id,
            source_address,
            destination_chain,
        });
        Ok(())
    }

    pub fn init_gateway_root(ctx: Context<InitGatewayRoot>) -> Result<()> {
        ctx.accounts.gateway_root_pda.set_inner(GatewayConfig {
            current_epoch: 0,
            previous_verifier_set_retention: 0,
            minimum_rotation_delay: 0,
            last_rotation_timestamp: 0,
            operator: ctx.accounts.funder.key(),
            domain_separator: [0u8; 32],
            bump: ctx.bumps.gateway_root_pda,
        });
        Ok(())
    }

    pub fn init_verification_session(
        ctx: Context<InitVerificationSession>,
        _payload_merkle_root: [u8; 32],
    ) -> Result<()> {
        ctx.accounts
            .verification_session_account
            .set_inner(VerificationSessionAccount {
                signature_verification: SignatureVerification {
                    accumulated_threshold: 0,
                    signature_slots: [0u8; 32],
                    signing_verifier_set_hash: [0u8; 32],
                },
                bump: ctx.bumps.verification_session_account,
            });
        Ok(())
    }

    pub fn interchain_transfer(
        ctx: Context<InterchainTransferCtx>,
        token_id: [u8; 32],
        source_address: Pubkey,
        source_token_account: Pubkey,
        destination_chain: String,
        destination_address: Vec<u8>,
        amount: u64,
        data_hash: [u8; 32],
    ) -> Result<()> {
        anchor_lang::prelude::emit_cpi!(InterchainTransfer {
            token_id,
            source_address,
            source_token_account,
            destination_chain,
            destination_address,
            amount,
            data_hash,
        });
        Ok(())
    }

    pub fn link_token_started(
        ctx: Context<LinkTokenStartedCtx>,
        token_id: [u8; 32],
        destination_chain: String,
        source_token_address: Pubkey,
        destination_token_address: Vec<u8>,
        token_manager_type: u8,
        params: Vec<u8>,
    ) -> Result<()> {
        anchor_lang::prelude::emit_cpi!(LinkTokenStarted {
            token_id,
            destination_chain,
            source_token_address,
            destination_token_address,
            token_manager_type,
            params,
        });
        Ok(())
    }

    pub fn interchain_token_deployment_started(
        ctx: Context<InterchainTokenDeploymentStartedCtx>,
        token_id: [u8; 32],
        token_name: String,
        token_symbol: String,
        token_decimals: u8,
        minter: Vec<u8>,
        destination_chain: String,
    ) -> Result<()> {
        anchor_lang::prelude::emit_cpi!(InterchainTokenDeploymentStarted {
            token_id,
            token_name,
            token_symbol,
            token_decimals,
            minter,
            destination_chain,
        });
        Ok(())
    }

    pub fn token_metadata_registered(
        ctx: Context<TokenMetadataRegisteredCtx>,
        token_address: Pubkey,
        decimals: u8,
    ) -> Result<()> {
        anchor_lang::prelude::emit_cpi!(TokenMetadataRegistered {
            token_address,
            decimals
        });
        Ok(())
    }

    pub fn signers_rotated(
        ctx: Context<SignersRotatedCtx>,
        epoch_le: [u8; 32],
        verifier_set_hash: [u8; 32],
    ) -> Result<()> {
        anchor_lang::prelude::emit_cpi!(VerifierSetRotatedEvent {
            epoch: U256(epoch_le),
            verifier_set_hash,
        });
        Ok(())
    }
}

#[event_cpi]
#[derive(Accounts)]
pub struct PayNativeForContractCall<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: This account is used as a configuration PDA for event emission only
    pub config_pda: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct RefundNativeFees<'info> {
    /// CHECK: This account is used as a configuration PDA for event emission only
    pub config_pda: UncheckedAccount<'info>,
    /// CHECK: This account is used as a receiver address for refund operations
    pub receiver: UncheckedAccount<'info>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct AddNativeGas<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,
    /// CHECK: This account is used as a configuration PDA for event emission only
    pub config_pda: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[event_cpi]
pub struct CallContract<'info> {
    /// The program that wants to call us - must be executable
    /// CHECK: Anchor constraint verifies this is an executable program
    pub calling_program: UncheckedAccount<'info>,
    /// The standardized PDA that must sign - derived from the calling program
    /// CHECK: This account is a PDA derived from the calling program for signing purposes
    pub signing_pda: UncheckedAccount<'info>,
    /// The gateway configuration PDA being initialized
    #[account()]
    pub gateway_root_pda: Account<'info, GatewayConfig>,
}

#[derive(Accounts)]
pub struct InitGatewayRoot<'info> {
    #[account(mut)]
    pub funder: Signer<'info>,
    #[account(
        init,
        payer = funder,
        space = 8 + std::mem::size_of::<GatewayConfig>(),
        seeds = [seed_prefixes::GATEWAY_SEED],
        bump
    )]
    pub gateway_root_pda: Account<'info, GatewayConfig>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(payload_merkle_root: [u8; 32])]
pub struct InitVerificationSession<'info> {
    #[account(mut)]
    pub funder: Signer<'info>,
    #[account(
        init,
        payer = funder,
        space = 8 + std::mem::size_of::<VerificationSessionAccount>(),
        seeds = [seed_prefixes::SIGNATURE_VERIFICATION_SEED, payload_merkle_root.as_ref()],
        bump
    )]
    pub verification_session_account: Account<'info, VerificationSessionAccount>,
    pub system_program: Program<'info, System>,
}

#[account]
#[derive(Debug, PartialEq, Eq)]
pub struct GatewayConfig {
    pub current_epoch: VerifierSetEpoch,
    pub previous_verifier_set_retention: VerifierSetEpoch,
    pub minimum_rotation_delay: RotationDelaySecs,
    pub last_rotation_timestamp: Timestamp,
    pub operator: Pubkey,
    pub domain_separator: [u8; 32],
    pub bump: u8,
}

pub type Timestamp = u64;
/// Seconds that need to pass between signer rotations
pub type RotationDelaySecs = u64;
/// Ever-incrementing idx for the signer set
pub type VerifierSetEpoch = u64;

// #[event_cpi]
// #[derive(Accounts)]
// pub struct ApproveMessage<'info> {
//     #[account(mut)]
//     pub payer: Signer<'info>,
// }

#[derive(Accounts)]
#[event_cpi]
#[instruction(message: MerkleisedMessage, payload_merkle_root: [u8; 32])]
pub struct ApproveMessage<'info> {
    #[account(
            seeds = [seed_prefixes::GATEWAY_SEED],
            bump = gateway_root_pda.bump
        )]
    pub gateway_root_pda: Account<'info, GatewayConfig>,
    #[account(mut)]
    pub funder: Signer<'info>,
    #[account(
            seeds = [seed_prefixes::SIGNATURE_VERIFICATION_SEED, payload_merkle_root.as_ref()],
            bump = verification_session_account.bump
        )]
    pub verification_session_account: Account<'info, VerificationSessionAccount>,
    #[account(
        init,
        payer = funder,
        space = 8 + std::mem::size_of::<IncomingMessage>(),
        seeds = [seed_prefixes::INCOMING_MESSAGE_SEED, message.leaf.message.command_id().as_ref()],
        bump
    )]
    pub incoming_message_pda: Account<'info, IncomingMessage>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[event_cpi]
pub struct ExecuteMessage<'info> {
    #[account(mut)]
    pub funder: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[event_cpi]
pub struct InterchainTransferCtx<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
}

#[derive(Accounts)]
#[event_cpi]
pub struct LinkTokenStartedCtx<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
}

#[derive(Accounts)]
#[event_cpi]
pub struct InterchainTokenDeploymentStartedCtx<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
}

#[derive(Accounts)]
#[event_cpi]
pub struct TokenMetadataRegisteredCtx<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
}

#[derive(Accounts)]
#[event_cpi]
pub struct SignersRotatedCtx<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
}

#[derive(Debug, Eq, PartialEq, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct MerkleisedMessage {
    /// The leaf node representing the message in the Merkle tree.
    pub leaf: MessageLeaf,

    /// The Merkle proof demonstrating the message's inclusion in the payload's
    /// Merkle tree.
    pub proof: Vec<u8>,
}

#[derive(Clone, PartialEq, Eq, Debug, AnchorDeserialize, AnchorSerialize)]
pub struct MessageLeaf {
    /// The message contained within this leaf node.
    pub message: Message,

    /// The position of this leaf within the Merkle tree.
    pub position: u16,

    /// The total number of leaves in the Merkle tree.
    pub set_size: u16,

    /// A domain separator used to ensure the uniqueness of hashes across
    /// different contexts.
    pub domain_separator: [u8; 32],

    /// The Merkle root of the signing verifier set, used for verifying
    /// signatures.
    pub signing_verifier_set: [u8; 32],
}

impl MessageLeaf {
    // placeholder hash function
    pub fn hash(&self) -> [u8; 32] {
        // Use borsh serialization (matches how Anchor serializes data)
        let data = self.try_to_vec().expect("Serialization should not fail");
        solana_program::keccak::hash(&data).to_bytes()
    }
}

impl Message {
    // placeholder hash function
    pub fn hash(&self) -> [u8; 32] {
        // Use borsh serialization (matches how Anchor serializes data)
        let data = self.try_to_vec().expect("Serialization should not fail");
        solana_program::keccak::hash(&data).to_bytes()
    }

    pub fn command_id(&self) -> [u8; 32] {
        let cc_id = &self.cc_id;
        let command_id =
            solana_program::keccak::hashv(&[cc_id.chain.as_bytes(), b"-", cc_id.id.as_bytes()]).0;
        return command_id;
    }
}

#[derive(Clone, PartialEq, Eq, Debug, AnchorDeserialize, AnchorSerialize)]
pub struct CrossChainId {
    /// The name or identifier of the source blockchain.
    pub chain: String,

    /// A unique identifier within the specified blockchain.
    pub id: String,
}

#[derive(Clone, PartialEq, Eq, Debug, AnchorDeserialize, AnchorSerialize)]
pub struct Message {
    /// The cross-chain identifier of the message
    pub cc_id: CrossChainId,

    /// The source address from which the message originates.
    pub source_address: String,

    /// The destination blockchain where the message is intended to be sent.
    pub destination_chain: String,

    /// The destination address on the target blockchain.
    pub destination_address: String,

    /// A 32-byte hash of the message payload, ensuring data integrity.
    pub payload_hash: [u8; 32],
}

pub type VerifierSetHash = [u8; 32];

#[account]
#[derive(Debug, PartialEq, Eq)]
pub struct VerificationSessionAccount {
    pub signature_verification: SignatureVerification,
    pub bump: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub struct SignatureVerification {
    pub accumulated_threshold: u128,
    pub signature_slots: [u8; 32],
    pub signing_verifier_set_hash: VerifierSetHash,
}

#[account]
#[derive(Debug, PartialEq, Eq)]
pub struct IncomingMessage {
    pub bump: u8,
    pub signing_pda_bump: u8,
    pub status: MessageStatus,
    pub message_hash: [u8; 32],
    pub payload_hash: [u8; 32],
}

pub mod seed_prefixes {
    /// The seed prefix for deriving Gateway Config PDA
    pub const GATEWAY_SEED: &[u8] = b"gateway";
    /// The seed prefix for deriving `VerifierSetTracker` PDAs
    pub const VERIFIER_SET_TRACKER_SEED: &[u8] = b"ver-set-tracker";
    /// The seed prefix for deriving signature verification PDAs
    pub const SIGNATURE_VERIFICATION_SEED: &[u8] = b"gtw-sig-verif";
    /// The seed prefix for deriving call contract signature verification PDAs
    pub const CALL_CONTRACT_SIGNING_SEED: &[u8] = b"gtw-call-contract";
    /// The seed prefix for deriving incoming message PDAs
    pub const INCOMING_MESSAGE_SEED: &[u8] = b"incoming message";
    /// The seed prefix for deriving message payload PDAs
    pub const MESSAGE_PAYLOAD_SEED: &[u8] = b"message-payload";
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct MessageStatus(u8);

impl MessageStatus {
    /// Creates a `MessageStatus` value which can be interpreted as "approved".
    #[must_use]
    pub const fn approved() -> Self {
        Self(0)
    }

    pub const fn executed() -> Self {
        Self(1)
    }

    pub fn is_approved(&self) -> bool {
        self.0 == 0
    }
}
