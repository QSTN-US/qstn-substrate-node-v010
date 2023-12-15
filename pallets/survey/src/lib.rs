#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unreachable_code)]
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

// #[cfg(feature = "runtime-benchmarks")]
// mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        log,
        pallet_prelude::*,
        traits::{fungible},
    };

    use frame_system::pallet_prelude::*;
    use sp_runtime::{
        traits::{CheckedAdd, CheckedDiv, CheckedSub},
    };

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    pub type AccountId<T> = <T as frame_system::Config>::AccountId;
    type BalanceOf<T> = <<T as Config>::NativeBalance as fungible::Inspect<AccountId<T>>>::Balance;

    // Type abstractions for easier potential later modification
    type SurveyId = u128;
    type OwnerId<T> = AccountId<T>;
    type FunderId<T> = AccountId<T>;
    type ParticipantId<T> = AccountId<T>;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        type NativeBalance: fungible::Inspect<Self::AccountId>
            + fungible::Mutate<Self::AccountId>
            + fungible::hold::Inspect<Self::AccountId>
            + fungible::hold::Mutate<Self::AccountId>
            + fungible::freeze::Inspect<Self::AccountId>
            + fungible::freeze::Mutate<Self::AccountId>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        // A new survey is created
        SurveyCreated {
            survey_id: SurveyId,
            owner_id: OwnerId<T>,
        },

        // A survey is funded
        SurveyFunded {
            survey_id: SurveyId,
            funded_amount: BalanceOf<T>,
            funder_id: FunderId<T>,
        },

        // A reward is claimed
        RewardClaimed {
            survey_id: SurveyId,
            participant_id: ParticipantId<T>,
            reward_amount: BalanceOf<T>,
        },

        // A participant is registered as having completed the survey
        NewParticipantRegistered {
            survey_id: SurveyId,
            participant_id: ParticipantId<T>,
        },

        // Status is update for a given survey
        SurveyStatusUpdated {
            survey_id: SurveyId,
            new_status: Status,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Trying to do operations on a survey which has not been created yet.
        SurveyNotCreated,
        /// Trying to create a survey which has already been created.
        SurveyAlreadyCreated,
        /// Trying to fund a survey which has already been funded. A survey can be funded only once.
        SurveyAlreadyFunded,
        /// Trying to claim a reward on a survey which has not been funded yet.
        SurveyNotFunded,
        /// Trying to fund a survey with an amount inferior to participant_limit
        FundingInferiorNumberParticipants,
        /// Trying to claim a reward for a participant who has already claimed their reward.
        ParticipantAlreadyRewarded,
        /// Trying to register a participant_id already registered.
        ParticipantAlreadyRegistered,
        /// Trying to claim a reward for a participant_id who is not registered as a participant.
        ParticipantNotRegistered,
        /// Trying to register a new participant for a survey which reached maximum participants limit already.
        MaxNumberOfParticipantsReached,
        /// Trying to fund a survey with more than the available balance of funder.
        NotEnoughBalanceForFunding,
        /// Trying to do operations on a survey while not being its owner.
        NotOwnerOfSurvey,
        /// Trying to register a participant on an inactive survey
        SurveyIsNotActive,
        /// Defensive Error: While trying to claim a reward for a participant, survey has not enough funds.
        DefensiveNotEnoughFundsInSurveyForReward,
        /// Defensive Error: Error when dividing for reward computation
        DefensiveErrorWhenDividing,
        /// Defensive Error: An overflow occured when the operation was supposed to be safe
        DefensiveUnexpectedOverflow,
    }

    // STRUCTS & ENUMS
    #[derive(Clone, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen, Debug)]
    pub enum Status {
        Active,
        Paused,
        Completed,
    }

    #[derive(Clone, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen, Debug)]
    #[scale_info(skip_type_params(T))]
    pub struct Survey<T: Config> {
        pub survey_id: SurveyId,
        pub owner_id: OwnerId<T>,
        pub participants_limit: BalanceOf<T>,
        pub number_participants: BalanceOf<T>,
        pub is_funded: bool,
        pub funded_amount: Option<BalanceOf<T>>,
        pub reward_amount: Option<BalanceOf<T>>,
        pub status: Status,
        // created_at ?
    }

    // STORAGE UNITS
    #[pallet::storage]
    #[pallet::getter(fn get_survey)]
    /// StorageMap which stores every survey created.
    ///
    /// Types:
    ///     Key: [`SurveyId`]
    ///     Value: [`Survey<T>`]
    pub type SurveysMap<T: Config> = StorageMap<_, Blake2_128Concat, SurveyId, Survey<T>>;

    #[pallet::storage]
    #[pallet::getter(fn is_participant)]
    /// StorageDoubleMap which stores for every survey the participants who submitted an answer.
    ///
    /// Types:
    ///     Key1: [`SurveyId`]
    ///     Key2: [`ParticipantId<T>`]
    ///     Value: [`bool`]
    pub type Participants<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        SurveyId,
        Blake2_128Concat,
        ParticipantId<T>,
        bool,
        ValueQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn is_participant_already_rewarded)]
    /// StorageDoubleMap which stores for every survey the participants who are already rewarded.
    ///
    /// Types:
    ///     Key1: [`SurveyId`]
    ///     Key2: [`ParticipantId<T>`]
    ///     Value: [`bool`]
    pub type ParticipantsRewarded<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        SurveyId,
        Blake2_128Concat,
        ParticipantId<T>,
        bool,
        ValueQuery,
    >;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create a new survey
        ///
        /// - `survey_id`: The off-chain computed unique id of the survey
        /// - `participants_limmit`: The max number of participants for this survey
        ///
        /// REQUIRES: Survey must not have been crated already
        ///
        /// Emits `SurveyCreated`
        #[pallet::call_index(0)]
        #[pallet::weight(u64::default())]
        pub fn create_survey(
            origin: OriginFor<T>,
            survey_id: SurveyId,
            participants_limit: BalanceOf<T>,
        ) -> DispatchResult {
            let owner_id = ensure_signed(origin)?;

            // Check if survey is not already created
            ensure!(
                SurveysMap::<T>::get(survey_id).is_none(),
                Error::<T>::SurveyAlreadyCreated
            );

            // Create the survey
            let new_survey = Survey {
                survey_id,
                owner_id: owner_id.clone(),
                participants_limit,
                number_participants: 0u32.into(),
                is_funded: false,
                funded_amount: None,
                reward_amount: None,
                status: Status::Active,
            };

            SurveysMap::<T>::insert(survey_id, new_survey);

            Self::deposit_event(Event::SurveyCreated {
                survey_id,
                owner_id,
            });

            Ok(())
        }

        /// Fund an existing survey
        ///
        /// - `survey_id`: the off-chain computed unique id of the survey
        /// - `fund_amount`: the amount the owner is willing to fund the survey
        ///
        /// REQUIRES: Survey has to be created already.
        /// REQUIRES: Survey should not be already funded.
        /// REQUIRES: Owner should have enough free balance.
        /// REQUIRES: Can only be called by survey owner.
        ///
        /// Emits `SurveyFunded`
        #[pallet::call_index(1)]
        #[pallet::weight(u64::default())]
        pub fn fund_survey(
            origin: OriginFor<T>,
            survey_id: SurveyId,
            fund_amount: BalanceOf<T>,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;

            let survey_option = SurveysMap::<T>::get(survey_id);

            // Check that survey is created
            match survey_option {
                None => Err(Error::<T>::SurveyNotCreated.into()),
                Some(survey) => {
                    // Check that caller is owner
                    ensure!(survey.owner_id == caller, Error::<T>::NotOwnerOfSurvey);

                    // Check that survey is not already funded
                    ensure!(!survey.is_funded, Error::<T>::SurveyAlreadyFunded);

                    // Check that funding amount is superior to participants_limit (otherwise reward_amount will be equal to 0)
                    ensure!(
                        survey.participants_limit <= fund_amount,
                        Error::<T>::FundingInferiorNumberParticipants
                    );

                    // Check that owner has enough balance for funding
                    let owner_balance: BalanceOf<T> =
                        <T::NativeBalance as fungible::Inspect<AccountId<T>>>::balance(
                            &survey.owner_id,
                        );
                    let new_owner_balance = owner_balance
                        .checked_sub(&fund_amount)
                        .ok_or(Error::<T>::NotEnoughBalanceForFunding)?;

                    // Update owner balance
                    let _ = <T::NativeBalance as fungible::Mutate<AccountId<T>>>::set_balance(
                        &survey.owner_id,
                        new_owner_balance,
                    );

                    // Compute reward amount
                    let reward_amount = fund_amount
                        .checked_div(&survey.participants_limit)
                        .ok_or(Error::<T>::DefensiveErrorWhenDividing)
                        .map_err(|e| {
                            #[cfg(test)]
                            panic!("defensive error happened: {:?}", e);

                            log::error!(target: "..", "defensive error happened: {:?}", e);
                            e
                        })?;

                    // Fund survey
                    let funded_survey = Survey {
                        is_funded: true,
                        funded_amount: Some(fund_amount),
                        reward_amount: Some(reward_amount),
                        ..survey
                    };
                    SurveysMap::<T>::insert(survey_id, funded_survey);

                    Self::deposit_event(Event::SurveyFunded {
                        survey_id,
                        funded_amount: fund_amount,
                        funder_id: caller,
                    });

                    Ok(())
                }
            }
        }

        /// Create a survey and fund it
        ///
        /// - `survey_id`: the off-chain computed unique id of the survey
        /// - `participants_limmit`: The max number of participants for this survey
        /// - `fund_amount`: the amount the owner is willing to fund the survey
        ///
        /// REQUIRES: Survey must not have been crated already
        /// REQUIRES: Survey has to be created already.
        /// REQUIRES: Survey should not be already funded.
        /// REQUIRES: Owner should have enough free balance.
        /// REQUIRES: Can only be called by survey owner.
        ///
        /// Emits `SurveyCreated`, `SurveyFunded`
        #[pallet::call_index(2)]
        #[pallet::weight(u64::default())]
        pub fn create_and_fund_survey(
            origin: OriginFor<T>,
            survey_id: SurveyId,
            participants_limit: BalanceOf<T>,
            fund_amount: BalanceOf<T>,
        ) -> DispatchResult {
            Self::create_survey(origin.clone(), survey_id, participants_limit)?;
            Self::fund_survey(origin, survey_id, fund_amount)?;
            Ok(())
        }

        /// Register the address of a participant who completed the survey
        ///
        /// - `survey_id`: the off-chain computed unique id of the survey
        /// - `participant_id`: the address of the participant
        ///
        /// REQUIRES: Survey has to be created already.
        /// REQUIRES: Can only be called by survey owner.
        /// REQUIRES: Participant should not be already registered.
        ///
        /// Emits `NewParticipantRegistered`
        #[pallet::call_index(3)]
        #[pallet::weight(u64::default())]
        pub fn register_participant(
            origin: OriginFor<T>,
            survey_id: SurveyId,
            participant_id: ParticipantId<T>,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;

            let survey_option = SurveysMap::<T>::get(survey_id);

            // Check that survey is created
            match survey_option {
                None => Err(Error::<T>::SurveyNotCreated.into()),
                Some(survey) => {
                    // Check that caller is owner
                    ensure!(survey.owner_id == caller, Error::<T>::NotOwnerOfSurvey);

                    // Check that survey is already funded
                    ensure!(survey.is_funded, Error::<T>::SurveyNotFunded);

                    // Check that participant is not already registered
                    ensure!(
                        !Self::is_participant(survey_id, participant_id.clone()),
                        Error::<T>::ParticipantAlreadyRegistered
                    );

                    // Check that we have not reached max number of participants already
                    ensure!(
                        survey.number_participants < survey.participants_limit,
                        Error::<T>::MaxNumberOfParticipantsReached
                    );

                    // Check that the survey is active
                    ensure!(
                        survey.status == Status::Active,
                        Error::<T>::SurveyIsNotActive
                    );

                    // Update participants storage unit
                    Participants::<T>::insert(survey_id, participant_id.clone(), true);

                    // Update number of participants
                    let number_participants = survey.number_participants + 1u32.into();

                    // Update number of participant on survey
                    let updated_survey = Survey {
                        number_participants,
                        ..survey
                    };
                    SurveysMap::<T>::insert(survey_id, updated_survey);

                    Self::deposit_event(Event::NewParticipantRegistered {
                        survey_id,
                        participant_id,
                    });

                    Ok(())
                }
            }
        }

        /// Claim reward on behalf of participant and update its balance
        ///
        /// - `survey_id`: the off-chain computed unique id of the survey
        /// - `participant_id`: the address of the participant
        ///
        /// REQUIRES: Survey has to be created already.
        /// REQUIRES: Can only be called by survey owner.
        /// REQUIRES: Participant should already be registered.
        /// REQUIRES: Reward should not have already been claimed.
        ///
        /// Emits `RewardClaimed`
        #[pallet::call_index(4)]
        #[pallet::weight(u64::default())]
        pub fn reward_participant(
            origin: OriginFor<T>,
            survey_id: SurveyId,
            participant_id: ParticipantId<T>,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;

            let survey_option = SurveysMap::<T>::get(survey_id);

            // Check that survey is created
            match survey_option {
                None => Err(Error::<T>::SurveyNotCreated.into()),
                Some(survey) => {
                    // Check that caller is owner
                    ensure!(survey.owner_id == caller, Error::<T>::NotOwnerOfSurvey);

                    // Check that survey is already funded
                    ensure!(survey.is_funded, Error::<T>::SurveyNotFunded);

                    // Check that participant is already registered
                    ensure!(
                        Self::is_participant(survey_id, participant_id.clone()),
                        Error::<T>::ParticipantNotRegistered
                    );

                    // Check that participant has not already been rewarded
                    ensure!(
                        !Self::is_participant_already_rewarded(survey_id, participant_id.clone()),
                        Error::<T>::ParticipantAlreadyRewarded
                    );

                    // Reward participant
                    let participant_balance: BalanceOf<T> =
                        <T::NativeBalance as fungible::Inspect<AccountId<T>>>::balance(
                            &participant_id,
                        );

                    // We can unwrap here as survey is verified to have been funded already.
                    let reward_amount = survey.reward_amount.unwrap_or_default();

                    let new_participant_balance = participant_balance
                        .checked_add(&reward_amount)
                        .ok_or(Error::<T>::DefensiveUnexpectedOverflow)
                        .map_err(|e| {
                            #[cfg(test)]
                            panic!("defensive error happened: {:?}", e);

                            log::error!(target: "..", "defensive error happened: {:?}", e);
                            e
                        })?;

                    // Update participant balance
                    let _ = <T::NativeBalance as fungible::Mutate<AccountId<T>>>::set_balance(
                        &participant_id,
                        new_participant_balance,
                    );

                    // Update reward storage unit
                    ParticipantsRewarded::<T>::insert(survey_id, participant_id.clone(), true);

                    Self::deposit_event(Event::RewardClaimed {
                        survey_id,
                        participant_id,
                        reward_amount,
                    });

                    Ok(())
                }
            }
        }

        /// Set the status of a survey
        ///
        /// - `survey_id`: the off-chain computed unique id of the survey
        /// - `status`: the address of the participant
        ///
        /// REQUIRES: Survey has to be created already.
        /// REQUIRES: Can only be called by survey owner.
        ///
        /// Emits `SurveyStatusUpdated`
        #[pallet::call_index(5)]
        #[pallet::weight(u64::default())]
        pub fn set_survey_status(
            origin: OriginFor<T>,
            survey_id: SurveyId,
            new_status: Status,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;

            let survey_option = SurveysMap::<T>::get(survey_id);

            // Check that survey is created
            match survey_option {
                None => Err(Error::<T>::SurveyNotCreated.into()),
                Some(survey) => {
                    // Check that caller is owner
                    ensure!(survey.owner_id == caller, Error::<T>::NotOwnerOfSurvey);

                    // Set new status
                    let survey_updated = Survey {
                        status: new_status.clone(),
                        ..survey
                    };

                    SurveysMap::<T>::insert(survey_id, survey_updated);

                    // Emit event
                    Self::deposit_event(Event::SurveyStatusUpdated {
                        survey_id,
                        new_status,
                    });

                    Ok(())
                }
            }
        }
    }

    impl<T: Config> Pallet<T> {}
}
