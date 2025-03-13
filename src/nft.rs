use crate::{
    constants::constants::{NFT_ROYALTIES, TOKEN_NAME, TOKEN_TICKER},
    errors::{ERR_INVALID_BET_ID, ERR_INVALID_NFT_TOKEN, ERR_INVALID_NFT_TOKEN_NONCE, ERR_INVALID_PAYMENT_COUNT, ERR_INVALID_ROLE, ERR_TOKEN_ALREADY_ISSUED, ERR_TOKEN_NOT_ISSUED},
    types::{Bet, BetAttributes, BetStatus, BetType}
};

multiversx_sc::imports!();

pub type AttributesAsMultiValue<M> =
    MultiValue7<u64, u64, BigUint<M>, BigUint<M>, BigUint<M>, BetType, BetStatus>;

#[multiversx_sc::module]
pub trait NftModule:
    crate::storage::StorageModule
    + crate::events::EventsModule
    + crate::utils::UtilsModule
{
    //--------------------------------------------------------------------------------------------//
    //-------------------------------- Token Issuance --------------------------------------------//
    //--------------------------------------------------------------------------------------------//

    /// Issues a new ESDT token for betting NFTs, callable only by the contract owner.
    /// Requires an EGLD payment to cover issuance costs.
    #[only_owner]
    #[payable("EGLD")]
    #[endpoint(issueToken)]
    fn issue_token(&self) {
        require!(self.bet_nft_token().is_empty(), ERR_TOKEN_ALREADY_ISSUED);
        let issue_cost = self.call_value().egld_value().clone_value();
        let token_name = ManagedBuffer::new_from_bytes(TOKEN_NAME);
        let token_ticker = ManagedBuffer::new_from_bytes(TOKEN_TICKER);

        self.bet_nft_token().issue_and_set_all_roles(
            EsdtTokenType::NonFungible,
            issue_cost,
            token_name,
            token_ticker,
            0,
            Some(self.callbacks().issue_callback())
        );
    }

    /// Callback function executed after token issuance to store the token ID.
    /// Parameters:
    /// - result: The result of the async call (success or error).
    #[callback]
    fn issue_callback(
        &self,
        #[call_result] result: ManagedAsyncCallResult<TokenIdentifier>,
    ) {
        match result {
            ManagedAsyncCallResult::Ok(token_id) => {
                self.bet_nft_token().set_token_id(token_id);
            }
            ManagedAsyncCallResult::Err(_) => { }
        }
    }

    /// Sets local roles for the NFT token, callable only by the contract owner.
    /// Requires the token to be issued first.
    #[only_owner]
    #[endpoint(setLocalRoles)]
    fn set_local_roles(&self) {
        require!(!self.bet_nft_token().is_empty(), ERR_TOKEN_NOT_ISSUED);

        let token = self.bet_nft_token().get_token_id();
        let roles = [EsdtLocalRole::NftUpdateAttributes];

        self.send()
            .esdt_system_sc_proxy()
            .set_special_roles(
                &self.blockchain().get_sc_address(),
                &token,
                roles.iter().cloned(),
            )
            .async_call_and_exit();
    }

    //--------------------------------------------------------------------------------------------//
    //-------------------------------- NFT Minting -----------------------------------------------//
    //--------------------------------------------------------------------------------------------//

    /// Mints a new NFT representing a bet and returns its nonce.
    /// Parameters:
    /// - bet: The bet object containing details to encode in the NFT.
    /// Returns: The nonce of the minted NFT.
    fn mint_bet_nft(&self, bet: &Bet<Self::Api>) -> u64 {
        require!(!self.bet_nft_token().is_empty(), ERR_TOKEN_NOT_ISSUED);
        let big_one = BigUint::from(1u64);

        let mut token_name = ManagedBuffer::new_from_bytes(b"Betslip #");
        let bet_id_buffer = self.u64_to_ascii(bet.bet_id);
        token_name.append(&bet_id_buffer);
        let royalties = BigUint::from(NFT_ROYALTIES);

        let mut uris = ManagedVec::new();
        uris.push(bet_id_buffer);

        let attributes = BetAttributes {
            event: bet.event,
            selection: bet.selection.clone(),
            stake: bet.stake_amount.clone(),
            potential_win: bet.potential_profit.clone(),
            odd: bet.odd.clone(),
            bet_type: bet.bet_type,
            status: bet.status,
        };
        let mut serialized_attributes = ManagedBuffer::new();
        if let Err(err) = attributes.top_encode(&mut serialized_attributes) {
            sc_panic!("Attributes encode error: {}", err.message_bytes());
        }

        let attributes_sha256 = self.crypto().sha256(&serialized_attributes);
        let attributes_hash = attributes_sha256.as_managed_buffer();

        self.send().esdt_nft_create(
            self.bet_nft_token().get_token_id_ref(),
            &big_one,
            &token_name,
            &royalties,
            &attributes_hash,
            &attributes,
            &uris,
        )
    }

    /// Retrieves a bet from storage by its ID.
    /// Parameters:
    /// - bet_id: The ID of the bet to retrieve.
    /// Returns: The Bet object if it exists.
    fn get_bet(&self, bet_id: u64) -> Bet<Self::Api> {
        let bet_mapper = self.bet_by_id(bet_id);
        require!(!bet_mapper.is_empty(), ERR_INVALID_BET_ID);
        bet_mapper.get()
    }
}