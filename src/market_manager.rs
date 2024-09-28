use crate::storage::{self, BetType, Market, Selection, Status};

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait MarketManagerModule: storage::StorageModule{
    
    #[only_owner]
    #[endpoint(createMarket)]
    fn create_market(
        &self,
        market_id: BigUint,
        event_id: BigUint,
        description: ManagedBuffer,
        selections: ManagedVec<Selection<Self::Api>>
        ){
        require!(self.markets(&market_id).is_empty(), "Market already exists");
        let market = Market {
            market_id: market_id.clone(),
            event_id: event_id,
            description,
            selections,
            back_liquidity: BigUint::zero(),
            lay_liquidity: BigUint::zero(),
            best_back_odds: BigUint::zero(),
            best_lay_odds: BigUint::zero(),
            bets: ManagedVec::new(),
        };
        self.markets(&market_id).set(&market);
    }

    #[only_owner]
    #[endpoint(closeMarket)]
    fn close_market(&self, market_id: BigUint, winning_selection_id: BigUint) {
        let mut market = self.markets(&market_id).get();
        for mut bet in market.bets.iter() {
            if bet.option == winning_selection_id {
            // Pariul este câștigător
                bet.status = Status::Win;
            } else {
            // Pariul este pierzător
                bet.status = Status::Lost;
            }
        }
        self.markets(&market_id).set(&market);
    }

}