use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    program::{invoke, invoke_signed},
    system_instruction,
    sysvar,
};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::{spl_token_2022, Token2022},
};
use mpl_token_metadata::{
    instructions::CreateV1CpiBuilder,
    types::{Creator, TokenStandard},
    ID as METADATA_PROGRAM_ID,
};

declare_id!("YOUR_PROGRAM_ID_HERE");

pub const BATCH_SEED: &[u8] = b"batch";
pub const MINT_SEED: &[u8] = b"mint";

#[program]
pub mod legendary_mint {
    use super::*;

    pub fn initialize_batch(
        ctx: Context<InitializeBatch>,
        ritual_hash: [u8; 32],
    ) -> Result<()> {
        let batch = &mut ctx.accounts.batch;
        batch.treasury = ctx.accounts.treasury.key();
        batch.ritual_hash = ritual_hash;
        batch.bump = *ctx.bumps.get("batch").unwrap();
        Ok(())
    }

    pub fn mint_legendary(
        ctx: Context<MintLegendary>,
        name: String,
        symbol: String,
        uri: String,
    ) -> Result<()> {
        let payer = ctx.accounts.payer.to_account_info();
        let mint = ctx.accounts.mint.to_account_info();
        let system_program = ctx.accounts.system_program.to_account_info();

        // ------------------------------------------------------------
        // 1. CREATE MINT ACCOUNT WITH EXTENSION SPACE
        // ------------------------------------------------------------
        let mint_len = spl_token_2022::extension::ExtensionType::try_calculate_account_len::<spl_token_2022::state::Mint>(
            &[spl_token_2022::extension::ExtensionType::NonTransferable],
        )?;

        invoke(
            &system_instruction::create_account(
                &payer.key(),
                &mint.key(),
                Rent::get()?.minimum_balance(mint_len),
                mint_len as u64,
                &spl_token_2022::id(),
            ),
            &[
                payer.clone(),
                mint.clone(),
                system_program.clone(),
            ],
        )?;

        // ------------------------------------------------------------
        // 2. INITIALIZE NON-TRANSFERABLE EXTENSION
        // ------------------------------------------------------------
        invoke(
            &spl_token_2022::extension::non_transferable::instruction::initialize_non_transferable_mint(
                &spl_token_2022::id(),
                &mint.key(),
            ),
            &[mint.clone()],
        )?;

        // ------------------------------------------------------------
        // 3. INITIALIZE MINT (DECIMALS = 0)
        // ------------------------------------------------------------
        invoke(
            &spl_token_2022::instruction::initialize_mint2(
                &spl_token_2022::id(),
                &mint.key(),
                &payer.key(),
                None,
                0,
            )?,
            &[mint.clone()],
        )?;

        // ------------------------------------------------------------
        // 4. CREATE ATA IF NEEDED
        // ------------------------------------------------------------
        anchor_spl::associated_token::create(CpiContext::new(
            ctx.accounts.associated_token_program.to_account_info(),
            anchor_spl::associated_token::Create {
                payer: payer.clone(),
                associated_token: ctx.accounts.token_account.to_account_info(),
                authority: payer.clone(),
                mint: mint.clone(),
                system_program: system_program.clone(),
                token_program: ctx.accounts.token_program.to_account_info(),
            },
        ))?;

        // ------------------------------------------------------------
        // 5. MINT EXACTLY ONE TOKEN
        // ------------------------------------------------------------
        invoke(
            &spl_token_2022::instruction::mint_to(
                &spl_token_2022::id(),
                &mint.key(),
                &ctx.accounts.token_account.key(),
                &payer.key(),
                &[],
                1,
            )?,
            &[
                mint.clone(),
                ctx.accounts.token_account.to_account_info(),
                payer.clone(),
            ],
        )?;

        // ------------------------------------------------------------
        // 6. REVOKE MINT AUTHORITY (SET TO NONE)
        // ------------------------------------------------------------
        invoke(
            &spl_token_2022::instruction::set_authority(
                &spl_token_2022::id(),
                &mint.key(),
                None,
                spl_token_2022::instruction::AuthorityType::MintTokens,
                &payer.key(),
                &[],
            )?,
            &[mint.clone(), payer.clone()],
        )?;

        // ------------------------------------------------------------
        // 7. CREATE METADATA VIA METAPLEX CREATE V1
        // ------------------------------------------------------------
        let creators = vec![Creator {
            address: ctx.accounts.payer.key(),
            verified: false,
            share: 100,
        }];

        CreateV1CpiBuilder::new(&ctx.accounts.metadata_program)
            .metadata(&ctx.accounts.metadata)
            .mint(&mint, true)
            .authority(&payer)
            .payer(&payer)
            .update_authority(&payer, true)
            .system_program(&system_program)
            .sysvar_instructions(&ctx.accounts.sysvar_instructions)
            .spl_token_program(&ctx.accounts.token_program)
            .name(name)
            .symbol(symbol)
            .uri(uri)
            .seller_fee_basis_points(0)
            .creators(creators)
            .is_mutable(false)
            .token_standard(TokenStandard::NonFungible)
            .invoke()?;

        Ok(())
    }
}

# ------------------------------------------------------------
# ACCOUNTS
# ------------------------------------------------------------

#[derive(Accounts)]
pub struct InitializeBatch<'info> {
    #[account(
        init,
        payer = payer,
        space = Batch::LEN,
        seeds = [BATCH_SEED],
        bump
    )]
    pub batch: Account<'info, Batch>,

    /// CHECK: stored only
    pub treasury: AccountInfo<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct MintLegendary<'info> {
    #[account(
        seeds = [BATCH_SEED],
        bump = batch.bump,
        has_one = treasury
    )]
    pub batch: Account<'info, Batch>,

    /// CHECK: validated by has_one
    pub treasury: AccountInfo<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: manually created mint
    #[account(mut)]
    pub mint: AccountInfo<'info>,

    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = mint,
        associated_token::authority = payer
    )]
    pub token_account: Account<'info, anchor_spl::token::TokenAccount>,

    /// CHECK: PDA derived from Metaplex
    #[account(mut)]
    pub metadata: AccountInfo<'info>,

    /// CHECK: must equal Metaplex program ID
    #[account(address = METADATA_PROGRAM_ID)]
    pub metadata_program: AccountInfo<'info>,

    /// CHECK: required by Metaplex
    pub sysvar_instructions: AccountInfo<'info>,

    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

# ------------------------------------------------------------
# STATE
# ------------------------------------------------------------

#[account]
pub struct Batch {
    pub treasury: Pubkey,
    pub ritual_hash: [u8; 32],
    pub bump: u8,
}

impl Batch {
    pub const LEN: usize = 8 + 32 + 1 + 8; // anchor discriminator + fields + padding
}

# ------------------------------------------------------------
# ERRORS
# ------------------------------------------------------------

#[error_code]
pub enum LegendaryError {
    #[msg("Invalid ritual hash")]
    InvalidRitualHash,
}
