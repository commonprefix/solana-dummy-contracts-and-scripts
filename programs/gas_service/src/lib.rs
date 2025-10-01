use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;

declare_id!("H9XpBVCnYxr7cHd66nqtD8RSTrKY6JC32XVu2zT2kBmP");

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

#[program]
pub mod gas_service {
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
