use crate::{constants::constants::{NFT_ROYALTIES, TOKEN_NAME, TOKEN_TICKER}, errors::{ERR_INVALID_NFT_TOKEN, ERR_INVALID_NFT_TOKEN_NONCE, ERR_INVALID_ROLE, ERR_TOKEN_ALREADY_ISSUED, ERR_TOKEN_NOT_ISSUED}, types::{Bet, BetAttributes, BetStatus, BetType}};

multiversx_sc::imports!();

extern crate alloc;
use alloc::string::ToString;

const IPFS_GATEWAY: &[u8] = "https://ipfs.io/ipfs/".as_bytes();
const METADATA_KEY_NAME: &[u8] = "metadata:".as_bytes();
const METADATA_FILE_EXTENSION: &[u8] = ".json".as_bytes();

pub type AttributesAsMultiValue<M> =
    MultiValue7<u64, u64, BigUint<M>, BigUint<M>, BigUint<M>, BetType, BetStatus>;

#[multiversx_sc::module]
pub trait NftModule:
    crate::storage::StorageModule
    + crate::events::EventsModule
{
    #[only_owner]
    #[payable("EGLD")]
    #[endpoint(issueToken)]
    fn issue_token(&self) {
        require!(self.bet_nft_token().is_empty(), ERR_TOKEN_ALREADY_ISSUED);
 
        let issue_cost = self.call_value().egld_value().clone_value();

        let token_name = ManagedBuffer::new_from_bytes(TOKEN_NAME);
        let token_ticker = ManagedBuffer::new_from_bytes(TOKEN_TICKER);

        self.bet_nft_token().issue_and_set_all_roles(EsdtTokenType::NonFungible, issue_cost, token_name, token_ticker, 0, Some(self.callbacks().issue_callback()));

    }

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

    fn mint_bet_nft(&self, bet: &Bet<Self::Api>) -> u64 {
        require!(!self.bet_nft_token().is_empty(), ERR_TOKEN_NOT_ISSUED);
        let big_one = BigUint::from(1u64);

        let mut token_name = ManagedBuffer::new_from_bytes(b"BetCube Ticket #");
        let bet_id_buffer = self.u64_to_ascii(bet.nft_nonce);
        token_name.append(&bet_id_buffer);
        let royalties = BigUint::from(NFT_ROYALTIES);

        let mut uris = ManagedVec::new();
        let metadata = self.build_metadata(bet.nft_nonce);

        let attributes = BetAttributes {
            event: bet.event.clone(),
            stake: bet.stake_amount.clone(),
            potential_win: bet.potential_profit.clone(),
            odd: bet.odd.clone(),
            bet_type: bet.bet_type.clone(),
            status: bet.status.clone(),
            metadata,
        };
        let mut serialized_attributes = ManagedBuffer::new();
        if let core::result::Result::Err(err) = attributes.top_encode(&mut serialized_attributes) {
            sc_panic!("Attributes encode error: {}", err.message_bytes());
        }

        let attributes_sha256 = self.crypto().sha256(&serialized_attributes);
        let attributes_hash = attributes_sha256.as_managed_buffer();

        let uri = self.build_uri(bet.nft_nonce);
        uris.push(uri);

        let nonce = self.send().esdt_nft_create(
            self.bet_nft_token().get_token_id_ref(),
            &big_one,
            &token_name,
            &royalties,
            &attributes_hash,
            &attributes,
            &uris,
        );
        nonce
    }
    

    fn require_valid_bet_nft(
        &self,
        bet_id: u64,
    ) -> Bet<Self::Api> {
        let caller = self.blockchain().get_caller();
        let payments = self.call_value().all_esdt_transfers().clone_value();
        let bet: Bet<<Self as ContractBase>::Api> = self.get_bet(bet_id);

        if payments.len() == 0 {
            require!(caller == bet.bettor, ERR_INVALID_ROLE);
        } else {
            require!(payments.len() == 1, "Invalid");
            let payment = payments.get(0);
            require!(
                self.bet_nft_token().get_token_id() == payment.token_identifier,
                ERR_INVALID_NFT_TOKEN
            );
            require!(bet.nft_nonce == payment.token_nonce, ERR_INVALID_NFT_TOKEN_NONCE);        
        }
        bet
    }

    fn u64_to_ascii(&self, number: u64) -> ManagedBuffer {
        let mut reversed_digits = ManagedVec::<Self::Api, u8>::new();
        let mut result = number.clone();

        while result > 0 {
            let digit = result % 10;
            result /= 10;

            let digit_char = match digit {
                0 => b'0',
                1 => b'1',
                2 => b'2',
                3 => b'3',
                4 => b'4',
                5 => b'5',
                6 => b'6',
                7 => b'7',
                8 => b'8',
                9 => b'9',
                _ => sc_panic!("invalid digit"),
            };

            reversed_digits.push(digit_char);
        }

        if &reversed_digits.len() == &0 {
            return ManagedBuffer::new_from_bytes(b"0");
        }

        let mut o = ManagedBuffer::new();

        for digit in reversed_digits.iter().rev() {
            o.append_bytes(&[digit]);
        }

        o
    }

    fn build_uri(&self, number: u64) -> ManagedBuffer {
        let mut uri = ManagedBuffer::new_from_bytes(IPFS_GATEWAY);
        let cid = self.image_cid().get();
        let slash = ManagedBuffer::from("/".as_bytes());
        let index_file = ManagedBuffer::new_from_bytes(number.to_string().as_bytes());
        let uri_extension = ManagedBuffer::from(".png".as_bytes());

        uri.append(&cid);
        uri.append(&slash);
        uri.append(&index_file);
        uri.append(&uri_extension);

        uri
    }

    fn build_metadata(&self, number: u64) -> ManagedBuffer {
        let metadata_key_name = ManagedBuffer::new_from_bytes(METADATA_KEY_NAME);
        let metadata_cid = self.metadata_cid().get();
        let slash = ManagedBuffer::from("/".as_bytes());
        let index_file = ManagedBuffer::new_from_bytes(number.to_string().as_bytes());
        let file_extension = ManagedBuffer::new_from_bytes(METADATA_FILE_EXTENSION);

        let mut metadata = ManagedBuffer::new();
        metadata.append(&metadata_key_name);
        metadata.append(&metadata_cid);
        metadata.append(&slash);
        metadata.append(&index_file);
        metadata.append(&file_extension);

        metadata
    }

    #[storage_mapper("imageCid")]
    fn image_cid(&self) -> SingleValueMapper<ManagedBuffer>;

    #[storage_mapper("metadataCid")]
    fn metadata_cid(&self) -> SingleValueMapper<ManagedBuffer>;


    #[view(getBetslipData)]
    fn get_bet(&self, bet_id: u64) -> Bet<Self::Api> {
        let bet_mapper = self.bet_by_id(bet_id);
        require!(!bet_mapper.is_empty(), "Invalid");
        bet_mapper.get()
    }
}