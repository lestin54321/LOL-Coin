use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

declare_id!("4LuztCDUrco5NtduFvK2V8JrTXWWBt2NefTpK3vaSGfB"); // Program ID

#[program]
pub mod lol_coin_staking {
    use super::*;

    /// Initializes the staking pool with a reward rate (tokens/sec)
    /// and a lock-up duration (in seconds) during which early unstake
    /// incurs Betrayal Points.
    pub fn initialize(
        ctx: Context<Initialize>,
        reward_rate: u64,
        lockup_duration: i64,
    ) -> Result<()> {
        let pool = &mut ctx.accounts.staking_pool;
        pool.total_staked = 0;
        pool.reward_rate = reward_rate;
        pool.last_update_time = Clock::get()?.unix_timestamp;
        pool.reward_per_token_stored = 0;
        pool.lockup_duration = lockup_duration;
        msg!(
            "Initialized staking pool: reward_rate={} tokens/sec, lockup_duration={}s",
            reward_rate,
            lockup_duration
        );
        Ok(())
    }

    /// Stake `amount` of LOL tokens. Updates rewards and
    /// records staking timestamp for lock-up and loyalty calculation.
    pub fn stake(ctx: Context<Stake>, amount: u64) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        let pool = &mut ctx.accounts.staking_pool;
        let user = &mut ctx.accounts.user_stake;

        // 1. Update pool and user rewards, loyalty
        pool.update_rewards(user)?;

        // 2. Transfer tokens from user → pool
        let cpi_accounts = Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.staking_token_account.to_account_info(),
            authority: ctx.accounts.user_authority.to_account_info(),
        };
        let cpi_ctx =
            CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        // 3. Update stake balances & timestamps
        user.amount_staked = user.amount_staked.checked_add(amount).unwrap();
        pool.total_staked = pool.total_staked.checked_add(amount).unwrap();
        user.last_staked_time = now;

        msg!("User staked {} LOL tokens", amount);
        Ok(())
    }

    /// Unstake `amount` of LOL tokens. If within lock-up,
    /// impose Betrayal Points, reducing loyalty and pending rewards.
    pub fn unstake(ctx: Context<Unstake>, amount: u64) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        let pool = &mut ctx.accounts.staking_pool;
        let user = &mut ctx.accounts.user_stake;

        require!(
            user.amount_staked >= amount,
            StakingError::InsufficientStake
        );

        // 1. Update pool and user rewards, loyalty
        pool.update_rewards(user)?;

        // 2. Check lock-up for early unstake penalty
        let lock_end = user
            .last_staked_time
            .checked_add(pool.lockup_duration)
            .unwrap();
        if now < lock_end {
            // amount of betrayal points = tokens early withdrawn
            user.betrayal_points = user.betrayal_points.checked_add(amount).unwrap();
            // reduce pending rewards and loyalty score
            user.rewards_earned = user
                .rewards_earned
                .saturating_sub(amount);
            user.loyalty_score = user
                .loyalty_score
                .saturating_sub(amount);
            msg!(
                "Early unstake: +{} Betrayal Points; rewards & loyalty reduced",
                amount
            );
        }

        // 3. Transfer tokens back to user
        let cpi_accounts = Transfer {
            from: ctx.accounts.staking_token_account.to_account_info(),
            to: ctx.accounts.user_token_account.to_account_info(),
            authority: ctx.accounts.pool_authority.to_account_info(),
        };
        let cpi_ctx =
            CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        // 4. Update stake balances
        user.amount_staked = user.amount_staked.checked_sub(amount).unwrap();
        pool.total_staked = pool.total_staked.checked_sub(amount).unwrap();

        msg!("User unstaked {} LOL tokens", amount);
        Ok(())
    }

    /// Claim accumulated rewards. Rewards factor in loyalty
    /// (Love Points) at time of claim.
    pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
        let pool = &mut ctx.accounts.staking_pool;
        let user = &mut ctx.accounts.user_stake;

        // 1. Update pool and user rewards, loyalty
        pool.update_rewards(user)?;

        let rewards = user.rewards_earned;
        require!(rewards > 0, StakingError::NoRewardsAvailable);

        // 2. Transfer reward tokens to user
        let cpi_accounts = Transfer {
            from: ctx.accounts.reward_token_account.to_account_info(),
            to: ctx.accounts.user_token_account.to_account_info(),
            authority: ctx.accounts.pool_authority.to_account_info(),
        };
        let cpi_ctx =
            CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        token::transfer(cpi_ctx, rewards)?;

        // 3. Reset pending rewards
        user.rewards_earned = 0;
        msg!("User claimed {} LOL rewards", rewards);
        Ok(())
    }
}

impl StakingPool {
    /// Updates global reward‐per‐token and user‐specific earned rewards,
    /// then accrues Love Points proportional to time staked.
    fn update_rewards(&mut self, user: &mut UserStake) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        let dt = now
            .checked_sub(self.last_update_time)
            .unwrap();
        if self.total_staked > 0 {
            // increase global reward per token
            let delta = (dt as u128)
                .checked_mul(self.reward_rate as u128)
                .unwrap()
                .checked_div(self.total_staked as u128)
                .unwrap() as u64;
            self.reward_per_token_stored = self
                .reward_per_token_stored
                .checked_add(delta)
                .unwrap();
        }
        self.last_update_time = now;

        // compute newly earned rewards for user
        let owed_per_token = self
            .reward_per_token_stored
            .checked_sub(user.reward_per_token_paid)
            .unwrap();
        let earned = (user.amount_staked as u128)
            .checked_mul(owed_per_token as u128)
            .unwrap() as u64;
        user.rewards_earned = user.rewards_earned.checked_add(earned).unwrap();

        // update user's reward‐per‐token snapshot
        user.reward_per_token_paid = self.reward_per_token_stored;

        // accrue Love Points = seconds staked since last update
        user.loyalty_score = user
            .loyalty_score
            .checked_add(dt as u64)
            .unwrap();

        Ok(())
    }
}

#[account]
pub struct StakingPool {
    total_staked: u64,
    reward_rate: u64,
    last_update_time: i64,
    reward_per_token_stored: u64,
    lockup_duration: i64,
}

#[account]
pub struct UserStake {
    amount_staked: u64,
    last_staked_time: i64,
    rewards_earned: u64,
    reward_per_token_paid: u64,
    loyalty_score: u64,     // Love Points
    betrayal_points: u64,   // Betrayal Points
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = initializer, space = 8 + 8*5 + 8)]
    pub staking_pool: Account<'info, StakingPool>,
    #[account(mut)]
    pub initializer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(mut)]
    pub staking_pool: Account<'info, StakingPool>,
    #[account(init_if_needed, payer = user_authority, space = 8 + 8*6 + 8)]
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
    /// Pool authority must be the PDA set in your program
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
    #[msg("Invalid reward rate; must be > 0.")]
    InvalidRewardRate,
    #[msg("Invalid lockup duration.")]
    InvalidLockupDuration,
}