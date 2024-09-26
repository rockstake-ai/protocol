use crate::{errors::{ERR_INVALID_ROLE, ERR_SEND_ONE_STREAM_NFT, ERR_TOKEN_ALREADY_ISSUED, ERR_TOKEN_NOT_ISSUED}, storage::{Betslip, BetslipAttributes}};

multiversx_sc::imports!();

const TOKEN_NAME: &[u8] = b"BetcubeTickets";
const TOKEN_TICKER: &[u8] = b"BET";

const NFT_ROYALTIES: u64 = 0_00;

#[multiversx_sc::module]
pub trait BetslipNftModule:
    crate::storage::StorageModule
    + crate::events::EventsModule
{
    #[only_owner]
    #[payable("EGLD")]
    #[endpoint(issueToken)]
    fn issue_token(&self) {
        require!(self.betslip_nft_token().is_empty(), ERR_TOKEN_ALREADY_ISSUED);
 
        let issue_cost = self.call_value().egld_value().clone_value();

        let token_name = ManagedBuffer::new_from_bytes(TOKEN_NAME);
        let token_ticker = ManagedBuffer::new_from_bytes(TOKEN_TICKER);

        self.betslip_nft_token().issue_and_set_all_roles(EsdtTokenType::NonFungible, issue_cost, token_name, token_ticker, 0, Some(self.callbacks().issue_callback()));

    }

    #[callback]
    fn issue_callback(
        &self,
        #[call_result] result: ManagedAsyncCallResult<TokenIdentifier>,
    ) {
        match result {
            ManagedAsyncCallResult::Ok(token_id) => {
                    self.betslip_nft_token().set_token_id(token_id);
            }
            ManagedAsyncCallResult::Err(_) => { }
        }
    }

    fn mint_betslip_nft(&self, betslip: &Betslip<Self::Api>) -> u64 {
        require!(!self.betslip_nft_token().is_empty(), ERR_TOKEN_NOT_ISSUED);
        let big_one = BigUint::from(1u64);

        let mut token_name = ManagedBuffer::new_from_bytes(b"BetCube Ticket #");
        let betslip_id_buffer = self.u64_to_ascii(betslip.nft_nonce);
        token_name.append(&betslip_id_buffer);

        let mut uris = ManagedVec::new();
        let mut full_uri = self.betslip_nft_base_uri().get();
        full_uri.append_bytes(b"/betslip/");
        full_uri.append(&betslip_id_buffer);
        full_uri.append_bytes(b"/nft");

        uris.push(full_uri);

        let royalties = BigUint::from(NFT_ROYALTIES);

        let attributes = BetslipAttributes {
            creator: betslip.creator.clone(),
            bets: betslip.bets.clone(),
            total_odd: betslip.total_odd.clone(),
            stake: betslip.stake.clone(),
            payout: betslip.payout.clone(),
            payment_token: betslip.payment_token.clone(),
            payment_nonce: betslip.payment_nonce,
            // status: betslip.status.clone(),
            is_paid: false,
        };
        let mut serialized_attributes = ManagedBuffer::new();
        if let core::result::Result::Err(err) = attributes.top_encode(&mut serialized_attributes) {
            sc_panic!("Attributes encode error: {}", err.message_bytes());
        }

        let attributes_sha256 = self.crypto().sha256(&serialized_attributes);
        let attributes_hash = attributes_sha256.as_managed_buffer();

        let nonce = self.send().esdt_nft_create(
            self.betslip_nft_token().get_token_id_ref(),
            &big_one,
            &token_name,
            &royalties,
            &attributes_hash,
            &attributes,
            &uris,
        );

        nonce
    }

    fn require_valid_betslip_nft(
        &self,
        betslip_id: u64,
    ) -> Betslip<Self::Api> {

        let caller = self.blockchain().get_caller();
        let payments = self.call_value().all_esdt_transfers().clone_value();
        let betslip = self.get_betslip(betslip_id);

        if payments.len() == 0 {
            require!(caller == betslip.creator, "Invalid role");
        } else {
            require!(payments.len() == 1, "Invalid");
            let payment = payments.get(0);
            require!(
                self.betslip_nft_token().get_token_id() == payment.token_identifier,
                "Invalid"
            );
            require!(betslip.nft_nonce == payment.token_nonce, "Invalid");        }

        // if required_role_opt.is_some() {
        //     let required_role = required_role_opt.into_option().unwrap();
        //     require!(required_role == stream_role, ERR_INVALID_ROLE);
        // }

        betslip
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
}