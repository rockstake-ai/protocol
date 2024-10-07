use crate::{types::{Bet, BetType, BetStatus}};


multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, PartialEq)]
pub enum EventResult {
    NotReported,
    Reported(u64), // ID-ul selecției câștigătoare
}

#[multiversx_sc::module]
pub trait BetValidationModule:
    crate::storage::StorageModule
    + crate::events::EventsModule
    + crate::nft_manager::NftManagerModule
{
    // Funcție pentru raportarea rezultatului unui eveniment
    #[only_owner]
    #[endpoint(reportEventResult)]
    fn report_event_result(&self, market_id: u64, winning_selection_id: u64) -> SCResult<()> {
        require!(!self.markets(&market_id).is_empty(), "Market doesn't exist!");
        let mut market = self.markets(&market_id).get();
        
        // require!(
        //     market.close_timestamp < self.blockchain().get_block_timestamp(),
        //     "Market is not closed yet"
        // );

        let result_exists = market.selections.iter().any(|s| s.selection_id == winning_selection_id);
        require!(result_exists, "Invalid winning selection ID");

        self.event_results(&market_id).set(&EventResult::Reported(winning_selection_id));
        
        Ok(())
    }

    // Funcție pentru procesarea pariurilor după raportarea rezultatului
    #[endpoint(processBets)]
    fn process_bets(&self, market_id: u64) -> SCResult<MultiValueEncoded<u64>> {
        require!(!self.markets(&market_id).is_empty(), "Market doesn't exist!");
        let mut market = self.markets(&market_id).get();
        
        let event_result = self.event_results(&market_id).get();
        require!(event_result != EventResult::NotReported, "Event result not reported yet");

        if let EventResult::Reported(winning_selection_id) = event_result {
            let mut processed_bets = MultiValueEncoded::new();

            for bet in market.bets.iter() {
                if bet.status != BetStatus::Matched {
                    continue;
                }

                let is_winner = match bet.bet_type {
                    BetType::Back => bet.selection.selection_id == winning_selection_id,
                    BetType::Lay => bet.selection.selection_id != winning_selection_id,
                };

                let new_status = if is_winner { BetStatus::Win } else { BetStatus::Lost };
                let mut updated_bet = bet.clone();
                updated_bet.status = new_status;

                // Actualizăm pariul în storage
                self.bet_by_id(bet.nft_nonce).set(&updated_bet);

                // Procesăm plățile pentru pariurile câștigătoare
                if is_winner {
                    self.process_winning_bet(&updated_bet);
                }

                processed_bets.push(bet.nft_nonce);
            }

            // Actualizăm piața cu pariurile procesate
            self.markets(&market_id).set(&market);

            Ok(processed_bets)
        } else {
            sc_error!("Unexpected event result state")
        }
    }

    // Funcție auxiliară pentru procesarea plăților pentru pariurile câștigătoare
    fn process_winning_bet(&self, bet: &Bet<Self::Api>) {
        let bettor = &bet.bettor;
        let token_identifier = &bet.payment_token;
        let win_amount = &bet.potential_profit;

        // Transferăm suma câștigată către pariator
        self.send().direct(
            &bettor,
            &token_identifier,
            bet.payment_nonce,
            &win_amount,
        );

        // Emitem un eveniment pentru câștig
        self.bet_won_event(
            &bettor,
            &bet.nft_nonce,
            &bet.event,
            &bet.selection.selection_id,
            &win_amount,
            &token_identifier,
            bet.payment_nonce,
        );
    }

    // Storage pentru rezultatele evenimentelor
    #[view(getEventResult)]
    #[storage_mapper("eventResults")]
    fn event_results(&self, market_id: &u64) -> SingleValueMapper<EventResult>;

    #[event("bet_won")]
    fn bet_won_event(
        &self,
        #[indexed] bettor: &ManagedAddress,
        #[indexed] nft_nonce: &u64,
        #[indexed] event_id: &u64,
        #[indexed] selection_id: &u64,
        #[indexed] win_amount: &BigUint,
        #[indexed] token_identifier: &EgldOrEsdtTokenIdentifier,
        #[indexed] token_nonce: u64,
    );
  
}