use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;

declare_id!("CJ9f8WFdm3q38pmg426xQf7uum7RqvrmS9R58usHwNX7");

/// Represents the event emitted when native gas is paid for a contract call.
#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GasPaidEvent {
    /// The sender/payer of gas
    pub sender: Pubkey,
    /// Destination chain on the Axelar network
    pub destination_chain: String,
    /// Destination address on the Axelar network
    pub destination_address: String,
    /// The payload hash for the event we're paying for
    pub payload_hash: [u8; 32],
    /// The amount paid
    pub amount: u64,
    /// The refund address
    pub refund_address: Pubkey,
    /// Optional SPL token account (sender)
    pub spl_token_account: Option<Pubkey>,
}

type MessageId = String;
/// Represents the event emitted when native gas is added.
#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GasAddedEvent {
    /// The sender/payer of gas
    pub sender: Pubkey,
    /// Message Id
    pub message_id: String,
    /// The amount added
    pub amount: u64,
    /// The refund address
    pub refund_address: Pubkey,
    /// Optional SPL token account (sender)
    pub spl_token_account: Option<Pubkey>,
}

/// Represents the event emitted when native gas is refunded.
#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GasRefundedEvent {
    /// The receiver of the refund
    pub receiver: Pubkey,
    /// Message Id
    pub message_id: String,
    /// The amount refunded
    pub amount: u64,
    /// Optional SPL token account (receiver)
    pub spl_token_account: Option<Pubkey>,
}

#[program]
pub mod gas_service {
    use super::*;

    pub fn cpi_call_contract(
        ctx: Context<CpiCallContract>,
        destination_chain: String,
        destination_contract_address: String,
        payload_hash: [u8; 32],
        payload: Vec<u8>,
    ) -> Result<()> {
        // Create the CPI context for calling program_tester's call_contract
        let cpi_program = ctx.accounts.program_tester_program.to_account_info();
        let cpi_accounts = program_tester::cpi::accounts::CallContract {
            calling_program: ctx.accounts.gas_service_program.to_account_info(),
            signing_pda: ctx.accounts.signing_pda.to_account_info(),
            gateway_root_pda: ctx.accounts.gateway_root_pda.to_account_info(),
            event_authority: ctx.accounts.event_authority.to_account_info(),
            program: ctx.accounts.program_tester_program.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        // Make the CPI call to program_tester's call_contract
        program_tester::cpi::call_contract(
            cpi_ctx,
            destination_chain,
            destination_contract_address,
            payload_hash,
            payload,
        )?;

        Ok(())
    }

    pub fn pay_native_for_contract_call(
        ctx: Context<PayNativeForContractCall>,
        destination_chain: String,
        destination_address: String,
        payload_hash: [u8; 32],
        amount: u64,
        refund_address: Pubkey,
    ) -> Result<()> {
        anchor_lang::prelude::emit_cpi!(GasPaidEvent {
            sender: ctx.accounts.payer.key(),
            destination_chain,
            destination_address,
            payload_hash,
            amount,
            refund_address,
            spl_token_account: None,
        });

        Ok(())
    }

    pub fn refund_native_fees(
        ctx: Context<RefundNativeFees>,
        message_id: String,
        amount: u64,
    ) -> Result<()> {
        anchor_lang::prelude::emit_cpi!(GasRefundedEvent {
            receiver: ctx.accounts.receiver.key(),
            message_id,
            amount,
            spl_token_account: None,
        });

        Ok(())
    }

    pub fn add_native_gas(
        ctx: Context<AddNativeGas>,
        message_id: String,
        amount: u64,
        refund_address: Pubkey,
    ) -> Result<()> {
        // Simply emit the event without any on-chain logic (mocked version)
        anchor_lang::prelude::emit_cpi!(GasAddedEvent {
            sender: ctx.accounts.sender.key(),
            message_id,
            amount,
            refund_address,
            spl_token_account: None,
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
pub struct CpiCallContract<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// The program_tester program we're calling via CPI
    /// CHECK: This is the program_tester program ID
    pub program_tester_program: UncheckedAccount<'info>,

    /// The gas_service program itself (used as calling_program)
    /// CHECK: This is the gas_service program, verified by constraint
    #[account(executable)]
    pub gas_service_program: UncheckedAccount<'info>,

    /// The signing PDA for the CPI call
    /// CHECK: This PDA is derived from the gas_service program
    pub signing_pda: UncheckedAccount<'info>,

    /// The gateway root PDA from program_tester
    /// CHECK: This is validated by the program_tester program
    pub gateway_root_pda: UncheckedAccount<'info>,

    /// Event authority for CPI event emission
    /// CHECK: This is the event authority PDA for event-cpi
    pub event_authority: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}
