multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::{constants::{NFT_AMOUNT, ROYALTIES_MAX},storage::{self, BetParticipant, P2PBet, ParticipationNFT}};

#[multiversx_sc::module]
pub trait P2PModule: storage::StorageModule     
    + crate::events::EventsModule{

    #[endpoint(createP2PBet)]
    #[payable("*")]
    fn create_p2p_bet(
        &self,
        event_details: ManagedBuffer,
        options: ManagedVec<ManagedBuffer>,
        odds: ManagedVec<BigUint>,
    ) {
        let caller = self.blockchain().get_caller();

        // Generăm un ID unic pentru pariu
        let binding = self.crypto().sha256(&event_details);
        let bet_id = binding.as_managed_buffer();

        // Verificăm dacă pariu există deja
        require!(
            !self.active_bets().contains_key(&bet_id),
            "Bet with this ID already exists!"
        );

        // Cream pariu nou
        let new_bet = P2PBet {
            bet_id: bet_id.clone(),
            creator: caller.clone(),
            event_details: event_details.clone(),
            options: options.clone(),
            odds: odds.clone(),
            total_pool: BigUint::zero(),
            participants: ManagedVec::new(),
            is_active: true,
            result_declared: false,
            winning_option: ManagedBuffer::new(),
        };

        // Salvăm pariu în storage
        self.p2p_bets(&bet_id).set(&new_bet);
        self.active_bets().insert(bet_id.clone(), true);

        // Emit event (opțional)
        self.event_create_p2p_bet(&bet_id, &caller, &event_details);
    }

    #[endpoint(joinP2PBet)]
    #[payable("*")]
    fn join_p2p_bet(
        &self,
        bet_id: ManagedBuffer,
        option_chosen: ManagedBuffer,
    ) {
        let caller = self.blockchain().get_caller();
        let payment = self.call_value().egld_value().clone_value();

        // Verificăm dacă pariu este activ
        require!(
            self.active_bets().contains_key(&bet_id),
            "Bet does not exist or is not active!"
        );

        let mut bet = self.p2p_bets(&bet_id).get();

        // Verificăm dacă opțiunea aleasă este validă
        require!(
            bet.options.contains(&option_chosen),
            "Invalid betting option!"
        );

        // Înregistrăm participantul
        let participant = BetParticipant {
            address: caller.clone(),
            option_chosen: option_chosen.clone(),
            stake: payment.clone(),
            nft_id: None,
        };

        // Actualizăm pariu
        bet.total_pool += &payment;
        bet.participants.push(participant.clone());

        // Salvăm modificările
        self.p2p_bets(&bet_id).set(&bet);

        // Emit NFT pentru participant (opțional)
        let nft_nonce = self.issue_participation_nft(&caller, &bet_id, &option_chosen, &payment);

        // Actualizăm NFT ID în participant
        let mut updated_participant = participant;
        updated_participant.nft_id = Some(nft_nonce);

        // Actualizăm lista de participanți
        let index = bet.participants.len() - 1;
        let _ = bet.participants.set(index, &updated_participant);

        // Salvăm din nou pariu cu participantul actualizat
        self.p2p_bets(&bet_id).set(&bet);

        // Emit event (opțional)
        self.event_join_p2p_bet(&bet_id, &caller, &option_chosen, &payment);
    }

    #[endpoint(finalizeP2PBet)]
    fn finalize_p2p_bet(
        &self,
        bet_id: ManagedBuffer,
        winning_option: ManagedBuffer,
    ) {
        let caller = self.blockchain().get_caller();

        // Doar creatorul pariului poate finaliza pariu
        let bet = self.p2p_bets(&bet_id).get();
        require!(
            bet.creator == caller,
            "Only the bet creator can finalize the bet!"
        );

        require!(bet.is_active, "Bet is already finalized!");

        // Actualizăm pariu
        let mut updated_bet = bet;
        updated_bet.is_active = false;
        updated_bet.result_declared = true;
        updated_bet.winning_option = winning_option.clone();

        // Calculăm câștigurile și distribuim fondurile
        let total_pool = updated_bet.total_pool.clone();
        let mut total_winning_stake = BigUint::zero();

        for participant in &updated_bet.participants {
            if participant.option_chosen == winning_option {
                total_winning_stake += &participant.stake;
            }
        }

        // Evităm divizarea la zero
        require!(
            total_winning_stake > BigUint::zero(),
            "No winners for this bet!"
        );

        // Distribuim câștigurile
        for participant in &updated_bet.participants {
            if participant.option_chosen == winning_option {
                let share = &participant.stake * &total_pool.clone() / &total_winning_stake;

                // Transferăm fondurile câștigătorului
                self.send().direct_egld(&participant.address, &share);
            }
        }

        // Salvăm modificările
        self.p2p_bets(&bet_id).set(&updated_bet);
        self.active_bets().remove(&bet_id);

        // Emit event (opțional)
        self.event_finalize_p2p_bet(&bet_id, &winning_option);
    }

    fn issue_participation_nft(
        &self,
        participant: &ManagedAddress,
        bet_id: &ManagedBuffer,
        option_chosen: &ManagedBuffer,
        stake: &BigUint,
    ) -> u64 {
        self.token_manager().require_issued_or_set();

        let nft_attributes = ParticipationNFT {
            bet_id: bet_id.clone(),
            option_chosen: option_chosen.clone(),
            stake: stake.clone(),
        };

        let amount = &BigUint::from(NFT_AMOUNT);
        let uris = ManagedVec::new(); // Poți adăuga URI-uri dacă dorești

        let nonce = self.send().esdt_nft_create::<ParticipationNFT<Self::Api>>(
            self.token_manager().get_token_id_ref(),
            amount,
            &ManagedBuffer::from("Participation NFT"),
            &BigUint::from(ROYALTIES_MAX),
            &ManagedBuffer::new(),
            &nft_attributes,
            &uris,
        );

        // Transferăm NFT-ul către participant
        self.send().direct_esdt(
            &participant,
            self.token_manager().get_token_id_ref(),
            nonce,
            amount,
        );

        nonce
    }
}