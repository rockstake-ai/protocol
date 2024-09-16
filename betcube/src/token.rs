// multiversx_sc::imports!();
// multiversx_sc::derive_imports!();

// use crate::{constants::{TOKEN_NAME, TOKEN_TICKER, NFT_ISSUE_COST},storage};

// #[multiversx_sc::module]
// pub trait TokenModule:  storage::StorageModule     
// + crate::events::EventsModule {
//     #[only_owner]
//     #[payable("EGLD")]
//     #[endpoint(issue)]
//     fn issue_token(&self) {
//         self.blockchain().check_caller_is_owner();
//         let manager = self.token_manager();

//         let token_name = ManagedBuffer::new_from_bytes(TOKEN_NAME);
//         let token_ticker = ManagedBuffer::new_from_bytes(TOKEN_TICKER);
        
//         require!(manager.is_empty(), "Token already issued!");
//         let amount = self.call_value().egld_value();
//         require!(
//             amount.clone_value() == BigUint::from(NFT_ISSUE_COST),
//             "Insufficient funds!");
//         self.token_manager().issue_and_set_all_roles(EsdtTokenType::NonFungible, amount.clone_value(), token_name, token_ticker, 0, Some(self.callbacks().issue_callback()));
//     }

//     #[callback]
//     fn issue_callback(
//         &self,
//         #[call_result] result: ManagedAsyncCallResult<TokenIdentifier>,
//     ) {
//         match result {
//             ManagedAsyncCallResult::Ok(token_id) => {
//                     self.token_manager().set_token_id(token_id);
//             }
//             ManagedAsyncCallResult::Err(_) => { }
//         }
//     }
// }