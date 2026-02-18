use anchor_lang::prelude::*;
use anchor_lang::system_program;
use anchor_spl::{
associated_token::AssociatedToken,
token_interface::{Mint, TokenAccount, TokenInterface},
};
use mpl_token_metadata::{
instructions::CreateV1CpiBuilder,
types::{DataV2, TokenStandard},
ID as METADATA_PROGRAM_ID,
};
use spl_token_2022::{
extension::ExtensionType,
instruction::AuthorityType,
state::Mint as SplMint,
};

declare_id!(“YourProgramIdHere1111111111111111111111111111”); // Replace after `anchor deploy`

// ─────────────────────────────────────────────────────────────────────────────
// Program
// ─────────────────────────────────────────────────────────────────────────────

#[program]
pub mod uliqs_legendary {
use super::*;

```
/// One-time setup: creates the Batch PDA that tracks mint progress.
///
/// * `max_mints`           – hard cap per batch (≤ 100 enforced below)
/// * `mint_price_lamports` – lamports charged per mint, forwarded to treasury
pub fn initialize_batch(
    ctx: Context<InitializeBatch>,
    max_mints: u8,
    mint_price_lamports: u64,
) -> Result<()> {
    require!(max_mints > 0 && max_mints <= 100, ErrorCode::InvalidBatchSize);
    require!(mint_price_lamports > 0, ErrorCode::InvalidPrice);

    let batch = &mut ctx.accounts.batch;
    batch.authority          = ctx.accounts.authority.key();
    batch.treasury           = ctx.accounts.treasury.key();
    batch.current_mints      = 0;
    batch.max_mints          = max_mints;
    batch.mint_price_lamports = mint_price_lamports;
    batch.triggered          = false;
    batch.bump               = ctx.bumps.batch;
    Ok(())
}

/// Mint a soulbound (non-transferable) NFT to `payer`.
///
/// Correct Token-2022 extension lifecycle:
///   1. system_program::create_account   – allocate space sized for extensions
///   2. initialize_non_transferable_mint – extension must come BEFORE init_mint
///   3. initialize_mint2                 – initialize the mint itself
///   4. init ATA via associated_token    – handled by Anchor constraint
///   5. mint_to (1 token)
///   6. revoke mint authority            – makes supply permanently fixed at 1
///   7. CreateV1 CPI to Metaplex         – attach on-chain metadata
pub fn mint_soulbound(
    ctx: Context<MintSoulbound>,
    name: String,
    uri: String,
) -> Result<()> {
    // ── Guards ──────────────────────────────────────────────────────────
    let batch = &mut ctx.accounts.batch;
    require!(!batch.triggered,                 ErrorCode::BatchTriggered);
    require!(batch.current_mints < batch.max_mints, ErrorCode::MaxMintsReached);
    require!(name.len() <= 32,  ErrorCode::NameTooLong);
    require!(uri.len()  <= 200, ErrorCode::UriTooLong);

    // ── 1. Forward mint fee to treasury ─────────────────────────────────
    system_program::transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.payer.to_account_info(),
                to:   ctx.accounts.treasury.to_account_info(),
            },
        ),
        batch.mint_price_lamports,
    )?;

    // ── 2. Calculate mint account size including NonTransferable extension
    let extensions  = [ExtensionType::NonTransferableMint];
    let mint_size   = ExtensionType::try_calculate_account_len::<SplMint>(&extensions)
        .map_err(|_| error!(ErrorCode::ExtensionError))?;
    let rent_lamports = Rent::get()?.minimum_balance(mint_size);

    // ── 3. Allocate the mint account ─────────────────────────────────────
    //    The mint keypair must be passed in as a signer so that Solana
    //    accepts us as the account creator.
    system_program::create_account(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::CreateAccount {
                from: ctx.accounts.payer.to_account_info(),
                to:   ctx.accounts.mint.to_account_info(),
            },
        ),
        rent_lamports,
        mint_size as u64,
        ctx.accounts.token_program.key,
    )?;

    // ── 4. Initialize NonTransferable extension BEFORE init_mint ─────────
    //    This is a mint-level extension: every token of this mint is
    //    permanently non-transferable (soulbound).
    let init_nt_ix = spl_token_2022::instruction::initialize_non_transferable_mint(
        ctx.accounts.token_program.key,
        ctx.accounts.mint.key,
    )
    .map_err(|_| error!(ErrorCode::ExtensionError))?;

    anchor_lang::solana_program::program::invoke(
        &init_nt_ix,
        &[ctx.accounts.mint.to_account_info()],
    )?;

    // ── 5. Initialize the mint itself ────────────────────────────────────
    //    Decimals = 0, authority = program authority (revoked after mint).
    let init_mint_ix = spl_token_2022::instruction::initialize_mint2(
        ctx.accounts.token_program.key,
        ctx.accounts.mint.key,
        &ctx.accounts.authority.key(),
        None,   // freeze authority: none
        0,      // decimals
    )
    .map_err(|_| error!(ErrorCode::ExtensionError))?;

    anchor_lang::solana_program::program::invoke(
        &init_mint_ix,
        &[ctx.accounts.mint.to_account_info()],
    )?;

    // ── 6. Mint exactly 1 token to the recipient's ATA ───────────────────
    //    The ATA is created by Anchor via `init_if_needed` in the context.
    let mint_to_ix = spl_token_2022::instruction::mint_to(
        ctx.accounts.token_program.key,
        ctx.accounts.mint.key,
        ctx.accounts.token_account.key,
        &ctx.accounts.authority.key(),
        &[&ctx.accounts.authority.key()],
        1,
    )
    .map_err(|_| error!(ErrorCode::ExtensionError))?;

    anchor_lang::solana_program::program::invoke_signed(
        &mint_to_ix,
        &[
            ctx.accounts.mint.to_account_info(),
            ctx.accounts.token_account.to_account_info(),
            ctx.accounts.authority.to_account_info(),
        ],
        &[],
    )?;

    // ── 7. Revoke mint authority → supply is now permanently 1 ───────────
    let revoke_ix = spl_token_2022::instruction::set_authority(
        ctx.accounts.token_program.key,
        ctx.accounts.mint.key,
        None,                           // new authority: None = revoke
        AuthorityType::MintTokens,
        &ctx.accounts.authority.key(),
        &[&ctx.accounts.authority.key()],
    )
    .map_err(|_| error!(ErrorCode::ExtensionError))?;

    anchor_lang::solana_program::program::invoke_signed(
        &revoke_ix,
        &[
            ctx.accounts.mint.to_account_info(),
            ctx.accounts.authority.to_account_info(),
        ],
        &[],
    )?;

    // ── 8. Attach Metaplex metadata ──────────────────────────────────────
    let data = DataV2 {
        name:                    name.clone(),
        symbol:                  "ULIQS".to_string(),
        uri:                     uri.clone(),
        seller_fee_basis_points: 0,
        creators:                None,
        collection:              None,
        uses:                    None,
    };

    CreateV1CpiBuilder::new(&ctx.accounts.metadata_program)
        .metadata(&ctx.accounts.metadata)
        .mint(&ctx.accounts.mint, false)            // false = mint is not a signer here
        .mint_authority(&ctx.accounts.authority)
        .payer(&ctx.accounts.payer)
        .update_authority(&ctx.accounts.authority, true)
        .system_program(&ctx.accounts.system_program)
        .sysvar_instructions(&ctx.accounts.sysvar_instructions)
        .spl_token_program(Some(&ctx.accounts.token_program))
        .token_standard(TokenStandard::NonFungible)
        .data(data)
        .is_mutable(false)                          // immutable: metadata is final
        .invoke()?;

    // ── 9. Update batch counter & optionally fire threshold event ────────
    batch.current_mints += 1;

    emit!(MintCompleted {
        batch:   batch.key(),
        minter:  ctx.accounts.payer.key(),
        mint:    ctx.accounts.mint.key(),
        number:  batch.current_mints,
        name,
        uri,
    });

    if batch.current_mints == batch.max_mints {
        batch.triggered = true;
        emit!(WellTriggered {
            batch:         batch.key(),
            total_minted:  batch.current_mints,
            message:       "Aqua Vita well funding threshold reached!".to_string(),
        });
    }

    Ok(())
}
```

}

// ─────────────────────────────────────────────────────────────────────────────
// Contexts
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Accounts)]
pub struct InitializeBatch<’info> {
/// Batch PDA — one per authority, seeded deterministically.
#[account(
init,
payer  = payer,
space  = Batch::LEN,
seeds  = [b”batch”, authority.key().as_ref()],
bump,
)]
pub batch: Account<’info, Batch>,

```
/// The wallet that will own / administrate this batch.
pub authority: Signer<'info>,

/// Pays for PDA creation (can be the same as authority).
#[account(mut)]
pub payer: Signer<'info>,

/// Treasury that receives mint fees — must match the address stored in
/// Batch so it cannot be swapped out in later mint calls.
/// CHECK: Validated by address storage; use a PDA or a trusted multisig.
#[account(mut)]
pub treasury: AccountInfo<'info>,

pub system_program: Program<'info, System>,
```

}

#[derive(Accounts)]
pub struct MintSoulbound<’info> {
/// Batch PDA — must belong to `authority` and its treasury must match.
#[account(
mut,
has_one = authority,
has_one = treasury,
seeds   = [b”batch”, authority.key().as_ref()],
bump    = batch.bump,
)]
pub batch: Account<’info, Batch>,

```
/// Fresh mint keypair — generated client-side, passed as signer so that
/// `system_program::create_account` can write to it.  Must be uninitialized.
/// CHECK: Created manually inside the instruction with correct Token-2022 sizing.
#[account(mut, signer)]
pub mint: AccountInfo<'info>,

/// Associated token account for `payer` — Anchor creates it if absent.
#[account(
    init_if_needed,
    payer                       = payer,
    associated_token::mint      = mint,
    associated_token::authority = payer,
    associated_token::token_program = token_program,
)]
pub token_account: InterfaceAccount<'info, TokenAccount>,

/// Batch authority — signs minting & metadata creation.
pub authority: Signer<'info>,

/// Transaction fee payer; also becomes the NFT holder.
#[account(mut)]
pub payer: Signer<'info>,

/// Mint fee destination — validated against the address stored in Batch.
/// CHECK: Address enforced by `has_one = treasury` on the Batch constraint.
#[account(mut)]
pub treasury: AccountInfo<'info>,

/// Metaplex metadata account — derived off-chain as:
///   `["metadata", METADATA_PROGRAM_ID, mint]`
/// CHECK: Derived and validated inside the Metaplex CPI builder.
#[account(
    mut,
    seeds  = [
        b"metadata",
        METADATA_PROGRAM_ID.as_ref(),
        mint.key().as_ref(),
    ],
    bump,
    seeds::program = METADATA_PROGRAM_ID,
)]
pub metadata: AccountInfo<'info>,

/// Sysvar Instructions — required by the Metaplex CreateV1 instruction.
/// CHECK: This is a well-known sysvar.
#[account(address = anchor_lang::solana_program::sysvar::instructions::ID)]
pub sysvar_instructions: AccountInfo<'info>,

/// Metaplex Token Metadata program.
/// CHECK: Address is pinned to the canonical Metaplex program ID.
#[account(address = METADATA_PROGRAM_ID)]
pub metadata_program: AccountInfo<'info>,

pub token_program:           Interface<'info, TokenInterface>,
pub associated_token_program: Program<'info, AssociatedToken>,
pub system_program:           Program<'info, System>,
```

}

// ─────────────────────────────────────────────────────────────────────────────
// State
// ─────────────────────────────────────────────────────────────────────────────

#[account]
pub struct Batch {
pub authority:           Pubkey,  // 32
pub treasury:            Pubkey,  // 32
pub current_mints:       u8,      //  1
pub max_mints:           u8,      //  1
pub mint_price_lamports: u64,     //  8
pub triggered:           bool,    //  1
pub bump:                u8,      //  1
}

impl Batch {
/// 8 (discriminator) + fields above
pub const LEN: usize = 8 + 32 + 32 + 1 + 1 + 8 + 1 + 1;
}

// ─────────────────────────────────────────────────────────────────────────────
// Events
// ─────────────────────────────────────────────────────────────────────────────

/// Emitted on every successful mint.
#[event]
pub struct MintCompleted {
pub batch:   Pubkey,
pub minter:  Pubkey,
pub mint:    Pubkey,
pub number:  u8,
pub name:    String,
pub uri:     String,
}

/// Emitted when the final NFT in the batch is minted.
#[event]
pub struct WellTriggered {
pub batch:        Pubkey,
pub total_minted: u8,
pub message:      String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Errors
// ─────────────────────────────────────────────────────────────────────────────

#[error_code]
pub enum ErrorCode {
#[msg(“Max mints reached for this batch”)]
MaxMintsReached,
#[msg(“Batch already triggered — all NFTs minted”)]
BatchTriggered,
#[msg(“max_mints must be between 1 and 100”)]
InvalidBatchSize,
#[msg(“mint_price_lamports must be greater than zero”)]
InvalidPrice,
#[msg(“Name must be 32 characters or fewer”)]
NameTooLong,
#[msg(“URI must be 200 characters or fewer”)]
UriTooLong,
#[msg(“Token-2022 extension instruction failed”)]
ExtensionError,
}
