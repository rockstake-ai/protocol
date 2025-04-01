use crate::types::{Sport, BetType};
multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait UtilsModule:
crate::storage::StorageModule {
    fn generate_unique_bet_hash(
        &self,
        caller: &ManagedAddress<Self::Api>,
        sport: &Sport,
        market_id: &u64,
        selection_id: &u64,
        odds: &BigUint<Self::Api>,
        bet_type: &BetType,
        token_identifier: &EgldOrEsdtTokenIdentifier<Self::Api>,
        token_nonce: u64,
        amount: &BigUint<Self::Api>
    ) -> ManagedBuffer<Self::Api> {
        let current_timestamp = self.blockchain().get_block_timestamp();
        let current_nonce = self.blockchain().get_block_nonce();
        
        let mut data = ManagedBuffer::new();
        
        data.append(&caller.as_managed_buffer());
        
        let sport_value = match sport {
            Sport::Football => 1u8,
            Sport::Basketball => 2u8,
            Sport::CounterStrike => 3u8,
            Sport::Dota => 4u8,
            Sport::LeagueOfLegends => 5u8,
        };
        data.append_bytes(&[sport_value]);
        data.append(&self.serialize_u64(market_id));
        data.append(&self.serialize_u64(selection_id));
        
        data.append(&odds.to_bytes_be_buffer());
        
        let bet_type_value = match bet_type {
            BetType::Back => 1u8,
            BetType::Lay => 2u8,
        };
        data.append_bytes(&[bet_type_value]);
        
        data.append(&token_identifier.clone().into_name().clone());
        data.append(&self.serialize_u64(&token_nonce));
        data.append(&amount.to_bytes_be_buffer());
        
        data.append(&self.serialize_u64(&current_timestamp));
        data.append(&self.serialize_u64(&current_nonce));
        
        let hash_bytes = self.crypto().sha256(&data);
        hash_bytes.as_managed_buffer().clone()
    }

    fn serialize_u64(&self, value: &u64) -> ManagedBuffer<Self::Api> {
        let mut buffer = ManagedBuffer::new();
        let bytes = value.to_be_bytes();
        buffer.append_bytes(&bytes);
        buffer
    }

    /// Converts a u64 number to an ASCII string representation.
    /// Parameters:
    /// - number: The number to convert.
    /// Returns: A ManagedBuffer containing the ASCII string.
    fn u64_to_ascii(&self, number: u64) -> ManagedBuffer<Self::Api> {
        let mut reversed_digits = ManagedVec::<Self::Api, u8>::new();
        let mut result = number;
        
        if result == 0 {
            return ManagedBuffer::new_from_bytes(b"0");
        }
        
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
                _ => sc_panic!("Invalid digit"),
            };
            reversed_digits.push(digit_char);
        }
        
        let mut output = ManagedBuffer::new();
        for i in (0..reversed_digits.len()).rev() {
            output.append_bytes(&[reversed_digits.get(i)]);
        }
        
        output
    }

    fn get_bet_id_hash(&self, bet_hash: &ManagedBuffer<Self::Api>) -> u64 {
        if self.bet_hash_to_id().contains_key(bet_hash) {
            return self.bet_hash_to_id().get(bet_hash).unwrap_or_default();
        }
        
        let current_timestamp = self.blockchain().get_block_timestamp();
        let current_nft_nonce = self.next_bet_id().get();
        
        let mut data = ManagedBuffer::new();
        data.append(bet_hash);
        data.append(&self.serialize_u64(&current_timestamp));
        data.append(&self.serialize_u64(&current_nft_nonce));
        
        let hash_bytes = self.crypto().sha256(&data);
        let mut all_bytes = [0u8; 32];
        hash_bytes.as_managed_buffer().load_to_byte_array(&mut all_bytes);
        
        let mut u32_bytes = [0u8; 4];
        for i in 0..4 {
            u32_bytes[i] = all_bytes[i];
        }
        
        let raw_id_part = u32::from_be_bytes(u32_bytes);
        
        let max_id: u64 = 9_000_000_000; 
        let id_part = (raw_id_part as u64 % max_id) + 1_000_000_000;
        
        let timestamp_part = current_timestamp % 1_000_000;
        let final_id = id_part + timestamp_part; 
        
        self.next_bet_id().set(current_nft_nonce + 1);
        self.bet_hash_to_id().insert(bet_hash.clone(), final_id);
        
        final_id
    }


}