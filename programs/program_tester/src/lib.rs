use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;

declare_id!("7RdSDLUUy37Wqc6s9ebgo52AwhGiw4XbJWZJgidQ1fJc");

#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NativeGasPaidForContractCallEvent {
    pub config_pda: Pubkey,
    pub destination_chain: String,
    pub destination_address: String,
    pub payload_hash: [u8; 32],
    pub refund_address: Pubkey,
    pub params: Vec<u8>,
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
    /// The log index
    pub log_index: u64,
    /// The receiver of the refund
    pub receiver: Pubkey,
    /// amount of SOL
    pub fees: u64,
}

#[program]
pub mod hello_world {
    use super::*;

    pub fn pay_native_for_contract_call(
        ctx: Context<PayNativeForContractCall>,
        destination_chain: String,
        destination_address: String,
        payload_hash: [u8; 32],
        refund_address: Pubkey,
        params: Vec<u8>,
        gas_fee_amount: u64,
    ) -> Result<()> {
        let config_pda_key = ctx.accounts.config_pda.key();

        anchor_lang::prelude::emit_cpi!(NativeGasPaidForContractCallEvent {
            config_pda: config_pda_key,
            destination_chain,
            destination_address,
            payload_hash,
            refund_address,
            params,
            gas_fee_amount,
        });

        Ok(())
    }
    pub fn refund_native_fees(
        ctx: Context<RefundNativeFees>,
        tx_hash: [u8; 64],
        log_index: u64,
        fees: u64,
    ) -> Result<()> {
        // TODO(v2) consider making this a utility function in program-utils
        // similar to transfer_lamports

        anchor_lang::prelude::emit_cpi!(NativeGasRefundedEvent {
            tx_hash,
            config_pda: ctx.accounts.config_pda.key(),
            log_index,
            receiver: ctx.accounts.receiver.key(),
            fees,
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
            sender_key: ctx.accounts.payer.key(),
            destination_chain,
            destination_contract_address,
            payload_hash,
            payload,
        });
        Ok(())
    }
}

#[event_cpi]
#[derive(Accounts)]
pub struct PayNativeForContractCall<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// The Gas service config PDA (unchecked for this example)
    pub config_pda: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct RefundNativeFees<'info> {
    /// The Gas service config PDA (unchecked for this dummy implementation)
    pub config_pda: UncheckedAccount<'info>,
    /// The receiver of the refund (unchecked for this dummy implementation)
    pub receiver: UncheckedAccount<'info>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct CallContract<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
}
