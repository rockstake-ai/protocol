use crate::types::{BetType, BetStatus};
multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait FundModule:
    crate::storage::StorageModule
    + crate::events::EventsModule
    + crate::nft::NftModule {

    #[only_owner]
    #[payable("*")]
    #[endpoint(distributeRewards)]
    fn distribute_rewards(
        &self,
        bet_id: u64,
    ) {
        let bet = self.require_valid_bet_nft(bet_id);
        require!(bet.status == BetStatus::Win, "Bet is not in winning state");

        let amount_to_distribute = match bet.bet_type {
            BetType::Back => bet.potential_profit.clone(),
            BetType::Lay => &bet.liability - &bet.potential_profit,
        };

        require!(amount_to_distribute > BigUint::zero(), "No winnings to distribute");

        let payment = EgldOrEsdtTokenPayment::new(
            bet.payment_token,
            bet.payment_nonce,
            amount_to_distribute,
        );

        self.send().direct(
            &bet.bettor,
            &payment.token_identifier,
            payment.token_nonce,
            &payment.amount,
        );

        //TODO() - UPDATE
        // let mut updated_bet = bet.clone();
        // updated_bet.matched_amount = BigUint::zero();
        // updated_bet.unmatched_amount = BigUint::zero();

        self.emit_reward_distributed_event(bet_id, &bet.bettor, &payment.amount);

    }

    fn emit_reward_distributed_event(
        &self,
        bet_id: u64,
        bettor: &ManagedAddress,
        amount: &BigUint,
    ) {
        self.reward_distributed_event(bet_id, bettor, amount);
    }
}
