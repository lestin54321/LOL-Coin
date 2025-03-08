use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

declare_id!("4LuztCDUrco5NtduFvK2V8JrTXWWBt2NefTpK3vaSGfB");

#[program]
pub mod lol_coin {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, total_supply: u64) -> Result<()> {
        let mint = &mut ctx.accounts.mint;
        mint.supply = total_supply;
        mint.authority = *ctx.accounts.authority.key;
        msg!("LOL-Coin Mint Created with Supply: {}", total_supply);
        Ok(())
    }

    pub fn stake(ctx: Context<Stake>, amount: u64) -> Result<()> {
        let cpi_accounts = Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.staking_pool.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, amount)?;
        ctx.accounts.user_stake.amount += amount;
        ctx.accounts.user_stake.start_time = Clock::get()?.unix_timestamp;
        msg!("User staked {} LOL-Coin", amount);
        Ok(())
    }

    pub fn unstake(ctx: Context<Unstake>) -> Result<()> {
        let cpi_accounts = Transfer {
            from: ctx.accounts.staking_pool.to_account_info(),
            to: ctx.accounts.user_token_account.to_account_info(),
            authority: ctx.accounts.pool_authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, ctx.accounts.user_stake.amount)?;
        msg!("User unstaked {} LOL-Coin", ctx.accounts.user_stake.amount);
        ctx.accounts.user_stake.amount = 0;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = authority, space = 8 + 32 + 8)]
    pub mint: Account<'info, Mint>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub staking_pool: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_stake: Account<'info, StakeInfo>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(mut)]
    pub staking_pool: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_stake: Account<'info, StakeInfo>,
    #[account(mut)]
    pub pool_authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

#[account]
pub struct StakeInfo {
    pub amount: u64,
    pub start_time: i64,
}
