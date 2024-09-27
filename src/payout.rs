use crate::{errors::{ERR_TOKEN_ALREADY_ISSUED, ERR_TOKEN_NOT_ISSUED, ERR_ZERO_CLAIM}, storage::{Betslip, BetslipAttributes}};

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait PayoutModule:
    crate::storage::StorageModule
    + crate::events::EventsModule
    + crate::betslip_nft::BetslipNftModule {
   
    #[view(balance)]
    fn balance(&self, betslip_id: u64) -> BigUint {
        let stream = self.get_betslip(betslip_id);
        // let streamed_amount = self.payout_amount(betslip_id);
        stream.payout
    }

    fn claim_from_betslip_internal(
        &self,
        betslip_id: u64,
    ) -> EgldOrEsdtTokenPayment {
        let mut betslip = self.require_valid_betslip_nft(betslip_id);
        let amount = self.balance(betslip_id);
        let caller = self.blockchain().get_caller();

        self.betslip_by_id(betslip_id).set(&betslip);

        let mut nft_attributes: BetslipAttributes<Self::Api> = self
            .betslip_nft_token()
            .get_token_attributes(betslip.nft_nonce);
        self.betslip_nft_token()
            .nft_update_attributes(betslip.nft_nonce, &nft_attributes);

        self.send().direct_esdt(
            &caller,
            self.betslip_nft_token().get_token_id_ref(),
            betslip.nft_nonce,
            &BigUint::from(1u32),
        );

        self.claim_from_betslip_event(betslip_id, &amount, &caller);

        EgldOrEsdtTokenPayment::new(betslip.payment_token, betslip.payment_nonce, amount.clone())
    }

    #[payable("*")]
    #[endpoint(payout)]
    fn claim_payout(&self, betslip_id: u64) {
        let payment = self.claim_from_betslip_internal(betslip_id);

        let caller = self.blockchain().get_caller();
        // Send claimed tokens
        self.send().direct(
            &caller,
            &payment.token_identifier,
            payment.token_nonce,
            &payment.amount,
        );
    }

}
