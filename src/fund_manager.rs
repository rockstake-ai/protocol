use crate::storage::{BetType, BetStatus};
multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait FundManagerModule:
    crate::storage::StorageModule
    + crate::events::EventsModule
    + crate::nft_manager::NftManagerModule {

    #[only_owner]
    #[payable("*")]
    #[endpoint(distributeRewards)]
    fn distribute_rewards(
        &self,
        bet_id: u64,
    ) {
        let mut bet = self.require_valid_bet_nft(bet_id);
        require!(bet.status == BetStatus::Win, "Bet is not winning state");
 
        let amount_to_distribute = match bet.status {
            BetStatus::Win => match bet.bet_type {
                BetType::Back => bet.win_amount.clone(),
                BetType::Lay => &bet.stake_amount - &bet.win_amount,
            },
            BetStatus::Unmatched => bet.stake_amount.clone(),
            _ => sc_panic!("Bet is not in a state eligible for distribution"),
        };

        require!(amount_to_distribute > BigUint::zero(), "No winnings to distribute");

        let payment = EgldOrEsdtTokenPayment::new(bet.payment_token, bet.payment_nonce, amount_to_distribute);

        self.send().direct(
            &bet.bettor,
            &payment.token_identifier,
            payment.token_nonce,
            &payment.amount,
        );
    }
}
