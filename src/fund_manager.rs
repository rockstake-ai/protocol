use crate::storage::{BetType, Status};
multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait FundManagerModule:
    crate::storage::StorageModule
    + crate::events::EventsModule
    + crate::nft_manager::NftManagerModule {

    #[only_owner]
    #[payable("*")]
    #[endpoint(distributeWinnings)]
    fn distribute_winnings(
        &self,
        bet_id: u64,
    ) {
        let mut bet = self.require_valid_bet_nft(bet_id);
        require!(bet.status == Status::Win, "Bet is not winning state");
 
        let amount_to_distribute = match bet.bet_type {
            BetType::Back => bet.win_amount.clone(),
            BetType::Lay => {&bet.stake_amount - &bet.win_amount
            },
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

    // #[only_owner]
    // #[endpoint(refundUnmatchedBet)]
    // fn refund_unmatched_bet(&self, bet_id: u64) {
    //     let bet = self.bet_by_id(bet_id).get();
    //     require!(bet.status == Status::Unmatched, "Bet is not unmatched");

    //     let betslip = self.bet_by_id(bet.nft_nonce).get();
    //     require!(betslip.status == Status::Unmatched, "Bet is not unmatched");

    //     let payment = self.claim_from_betslip_internal(bet_id);
    //     let caller = self.blockchain().get_caller();

    //     self.send().direct(
    //         &caller,
    //         &payment.token_identifier,
    //         payment.token_nonce,
    //         &payment.amount,
    //     );

    //     let mut updated_bet = bet;
    //     updated_bet.status = Status::Canceled;
    //     self.bet_by_id(bet_id).set(&updated_bet);

    //     let mut updated_betslip = betslip;
    //     updated_betslip.status = Status::Canceled;
    //     self.bet_by_id(bet.nft_nonce).set(&updated_betslip);

    //     let mut nft_attributes: BetAttributes<Self::Api> = self
    //         .bet_nft_token()
    //         .get_token_attributes(betslip.nft_nonce);
    //     nft_attributes.status = Status::Canceled;
    //     self.bet_nft_token()
    //         .nft_update_attributes(betslip.nft_nonce, &nft_attributes);

    //     self.refund_unmatched_bet(bet_id, &bet.stake_amount, &betslip.bettor);
    // }
}