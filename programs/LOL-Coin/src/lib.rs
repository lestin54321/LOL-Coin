use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

declare_id!("4LuztCDUrco5NtduFvK2V8JrTXWWBt2NefTpK3vaSGfB"); // Program ID

#[program]
pub mod lol_coin_staking {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, reward_rate: u64) -> Result<()> {
        let staking_pool = &mut ctx.accounts.staking_pool;
        staking_pool.total_staked = 0;
        staking_pool.reward_rate = reward_rate;
        staking_pool.last_update_time = Clock::get()?.unix_timestamp;
        staking_pool.reward_per_token_stored = 0;
        msg!("Staking contract initialized with reward rate: {}", reward_rate);
        Ok(())
    }

    pub fn stake(ctx: Context<Stake>, amount: u64) -> Result<()> {
        let staking_pool = &mut ctx.accounts.staking_pool;
        let user = &mut ctx.accounts.user_stake;

        // Update rewards before changing stake
        staking_pool.update_rewards(&user)?;

        // Transfer tokens from the user to the staking pool
        let cpi_accounts = Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.staking_token_account.to_account_info(),
            authority: ctx.accounts.user_authority.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        user.amount_staked += amount;
        staking_pool.total_staked += amount;
        user.last_staked_time = Clock::get()?.unix_timestamp;
        msg!("User staked {} LOL tokens", amount);
        Ok(())
    }

    pub fn unstake(ctx: Context<Unstake>, amount: u64) -> Result<()> {
        let staking_pool = &mut ctx.accounts.staking_pool;
        let user = &mut ctx.accounts.user_stake;

        require!(user.amount_staked >= amount, StakingError::InsufficientStake);

        staking_pool.update_rewards(&user)?;

        // Transfer tokens back to the user
        let cpi_accounts = Transfer {
            from: ctx.accounts.staking_token_account.to_account_info(),
            to: ctx.accounts.user_token_account.to_account_info(),
            authority: ctx.accounts.pool_authority.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        user.amount_staked -= amount;
        staking_pool.total_staked -= amount;
        msg!("User unstaked {} LOL tokens", amount);
        Ok(())
    }

    pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
        let staking_pool = &mut ctx.accounts.staking_pool;
        let user = &mut ctx.accounts.user_stake;

        staking_pool.update_rewards(&user)?;

        let rewards = user.rewards_earned;
        require!(rewards > 0, StakingError::NoRewardsAvailable);

        // Transfer reward tokens to user
        let cpi_accounts = Transfer {
            from: ctx.accounts.reward_token_account.to_account_info(),
            to: ctx.accounts.user_token_account.to_account_info(),
            authority: ctx.accounts.pool_authority.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        token::transfer(cpi_ctx, rewards)?;

        user.rewards_earned = 0;
        msg!("User claimed {} LOL rewards", rewards);
        Ok(())
    }
}

#[account]
pub struct StakingPool {
    total_staked: u64,
    reward_rate: u64,
    last_update_time: i64,
    reward_per_token_stored: u64,
}

#[account]
pub struct UserStake {
    amount_staked: u64,
    last_staked_time: i64,
    rewards_earned: u64,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = initializer, space = 8 + 48)]
    pub staking_pool: Account<'info, StakingPool>,
    #[account(mut)]
    pub initializer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(mut)]
    pub staking_pool: Account<'info, StakingPool>,
    #[account(init_if_needed, payer = user_authority, space = 8 + 40)]
    pub user_stake: Account<'info, UserStake>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub staking_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(mut)]
    pub staking_pool: Account<'info, StakingPool>,
    #[account(mut)]
    pub user_stake: Account<'info, UserStake>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub staking_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub pool_authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    #[account(mut)]
    pub staking_pool: Account<'info, StakingPool>,
    #[account(mut)]
    pub user_stake: Account<'info, UserStake>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub reward_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub pool_authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

#[error_code]
pub enum StakingError {
    #[msg("Not enough tokens staked to perform this action.")]
    InsufficientStake,
    #[msg("No rewards available to claim.")]
    NoRewardsAvailable,
}

