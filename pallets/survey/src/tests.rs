use crate::{mock::*, AccountId, Config, Event, Status, Survey};
use codec::Encode;
use frame_support::{
    assert_noop, assert_ok,
    traits::{
        fungible::{self},
        OnFinalize, OnInitialize,
    },
};
use sp_runtime::BoundedVec;

// Utils

fn initialize_state() -> (crate::mock::AccountId, crate::mock::AccountId) {
    // Go past genesis block so events get deposited
    System::set_block_number(1);
    // Need to mint some token for testing
    for i in 1..10 {
        assert_ok!(<<Test as Config>::NativeBalance as fungible::Mutate<
            AccountId<Test>,
        >>::mint_into(&i, 1000000000));
    }
    (1, 2)
}

fn get_events() -> Vec<Event<Test>> {
    let evt = System::events()
        .into_iter()
        .map(|evt| evt.event)
        .collect::<Vec<_>>();
    let evt_pallet = evt.into_iter().filter_map(|event| {
        if let RuntimeEvent::PalletSurvey(inner) = event {
            Some(inner)
        } else {
            None
        }
    });

    evt_pallet.collect()
}

fn get_survey(survey_id: SurveyId) -> Survey<Test> {
    let survey = PalletSurvey::get_survey(survey_id);
    assert!(survey.is_some());
    survey.unwrap()
}

// UNIT TESTS

// create_survey
#[test]
fn create_new_survey_success() {
    new_test_ext().execute_with(|| {
        let (survey_owner, _participant) = initialize_state();
        let survey_id: SurveyId = 0;
        let participants_limit: ParticipantLimitType = 1000000;
        assert_ok!(PalletSurvey::create_survey(
            RuntimeOrigin::signed(survey_owner),
            survey_id,
            participants_limit
        ));

        // Test events
        let mut events = get_events();
        assert_eq!(
            events.pop(),
            Some(Event::SurveyCreated {
                survey_id,
                owner_id: survey_owner
            })
        );
    });
}

#[test]
fn create_new_survey_fail_already_existing() {
    new_test_ext().execute_with(|| {
        let (survey_owner, _participant) = initialize_state();
        let survey_id: SurveyId = 0;
        let participants_limit: ParticipantLimitType = 1000000;
        assert_ok!(PalletSurvey::create_survey(
            RuntimeOrigin::signed(survey_owner),
            survey_id,
            participants_limit
        ));

        assert_noop!(
            PalletSurvey::create_survey(
                RuntimeOrigin::signed(survey_owner),
                survey_id,
                participants_limit
            ),
            crate::Error::<Test>::SurveyAlreadyCreated
        );
    });
}

// fund_survey

#[test]
fn fund_survey_success() {
    new_test_ext().execute_with(|| {
        let (survey_owner, _participant) = initialize_state();
        let survey_id: SurveyId = 0;
        let participants_limit: ParticipantLimitType = 1000000;
        assert_ok!(PalletSurvey::create_survey(
            RuntimeOrigin::signed(survey_owner),
            survey_id,
            participants_limit
        ));

        let fund_amount = 1000000;

        assert_ok!(PalletSurvey::fund_survey(
            RuntimeOrigin::signed(survey_owner),
            survey_id,
            fund_amount
        ));

        // Test events
        let mut events = get_events();
        assert_eq!(
            events.pop(),
            Some(Event::SurveyFunded {
                survey_id,
                funder_id: survey_owner,
                funded_amount: 1000000
            })
        );
    });
}

#[test]
fn fund_survey_gives_expected_reward_amount_10000_for_1000() {
    new_test_ext().execute_with(|| {
        let (survey_owner, _participant) = initialize_state();
        let survey_id: SurveyId = 0;
        let participants_limit: ParticipantLimitType = 1000;
        assert_ok!(PalletSurvey::create_survey(
            RuntimeOrigin::signed(survey_owner),
            survey_id,
            participants_limit
        ));

        let fund_amount = 10000;

        assert_ok!(PalletSurvey::fund_survey(
            RuntimeOrigin::signed(survey_owner),
            survey_id,
            fund_amount
        ));

        let survey = get_survey(survey_id);
        let reward_amount_computed = survey.reward_amount.unwrap();
        println!("{:?}", reward_amount_computed);
        assert_eq!(reward_amount_computed, fund_amount / participants_limit);
        assert_eq!(reward_amount_computed, 10);
    });
}

#[test]
fn fund_survey_fails_funding_inferior_participants_limit() {
    new_test_ext().execute_with(|| {
        let (survey_owner, _participant) = initialize_state();
        let survey_id: SurveyId = 0;
        let participants_limit: ParticipantLimitType = 1000;
        assert_ok!(PalletSurvey::create_survey(
            RuntimeOrigin::signed(survey_owner),
            survey_id,
            participants_limit
        ));

        let fund_amount = 100;

        assert_noop!(
            PalletSurvey::fund_survey(RuntimeOrigin::signed(survey_owner), survey_id, fund_amount),
            crate::Error::<Test>::FundingInferiorNumberParticipants
        );
    });
}

#[test]
fn fund_survey_fails_survey_not_created() {
    new_test_ext().execute_with(|| {
        let (survey_owner, _participant) = initialize_state();
        let survey_id: SurveyId = 0;
        // let participants_limit: ParticipantLimitType = 1000;
        // assert_ok!(PalletSurvey::create_survey(
        //     RuntimeOrigin::signed(survey_owner),
        //     survey_id,
        //     participants_limit
        // ));

        let fund_amount = 100;

        assert_noop!(
            PalletSurvey::fund_survey(RuntimeOrigin::signed(survey_owner), survey_id, fund_amount),
            crate::Error::<Test>::SurveyNotCreated
        );
    });
}

#[test]
fn fund_survey_fails_survey_already_funded() {
    new_test_ext().execute_with(|| {
        let (survey_owner, _participant) = initialize_state();
        let survey_id: SurveyId = 0;
        let participants_limit: ParticipantLimitType = 1000;
        assert_ok!(PalletSurvey::create_survey(
            RuntimeOrigin::signed(survey_owner),
            survey_id,
            participants_limit
        ));

        let fund_amount = 1000;

        assert_ok!(PalletSurvey::fund_survey(
            RuntimeOrigin::signed(survey_owner),
            survey_id,
            fund_amount
        ));

        assert_noop!(
            PalletSurvey::fund_survey(RuntimeOrigin::signed(survey_owner), survey_id, fund_amount),
            crate::Error::<Test>::SurveyAlreadyFunded
        );
    });
}

#[test]
fn fund_survey_fails_survey_not_owner() {
    new_test_ext().execute_with(|| {
        let (survey_owner, _participant) = initialize_state();
        let survey_id: SurveyId = 0;
        let participants_limit: ParticipantLimitType = 1000;
        assert_ok!(PalletSurvey::create_survey(
            RuntimeOrigin::signed(survey_owner),
            survey_id,
            participants_limit
        ));

        let fund_amount = 100;

        assert_noop!(
            PalletSurvey::fund_survey(RuntimeOrigin::signed(_participant), survey_id, fund_amount),
            crate::Error::<Test>::NotOwnerOfSurvey
        );
    });
}

#[test]
fn fund_survey_fails_survey_not_enough_balance() {
    new_test_ext().execute_with(|| {
        let (survey_owner, _participant) = initialize_state();
        let survey_id: SurveyId = 0;
        let participants_limit: ParticipantLimitType = 1000;
        assert_ok!(PalletSurvey::create_survey(
            RuntimeOrigin::signed(survey_owner),
            survey_id,
            participants_limit
        ));

        let fund_amount = 1000000001;

        assert_noop!(
            PalletSurvey::fund_survey(RuntimeOrigin::signed(survey_owner), survey_id, fund_amount),
            crate::Error::<Test>::NotEnoughBalanceForFunding
        );
    });
}

// create_and_fud_survey
#[test]
fn create_and_fund_survey_success() {
    new_test_ext().execute_with(|| {
        let (survey_owner, _participant) = initialize_state();
        let survey_id: SurveyId = 0;
        let participants_limit: ParticipantLimitType = 1000000;
        let fund_amount = 1000000;

        assert_ok!(PalletSurvey::create_and_fund_survey(
            RuntimeOrigin::signed(survey_owner),
            survey_id,
            participants_limit,
            fund_amount
        ));

        // Test events
        let mut events = get_events();
        assert_eq!(
            events.pop(),
            Some(Event::SurveyFunded {
                survey_id,
                funder_id: survey_owner,
                funded_amount: 1000000
            })
        );
        assert_eq!(
            events.pop(),
            Some(Event::SurveyCreated {
                survey_id,
                owner_id: survey_owner
            })
        );
    });
}

// register_participant
#[test]
fn register_participant_success() {
    new_test_ext().execute_with(|| {
        let (survey_owner, participant_id) = initialize_state();
        let survey_id: SurveyId = 0;
        let participants_limit: ParticipantLimitType = 1000000;
        let fund_amount = 1000000;

        assert_ok!(PalletSurvey::create_and_fund_survey(
            RuntimeOrigin::signed(survey_owner),
            survey_id,
            participants_limit,
            fund_amount
        ));

        assert!(!PalletSurvey::is_participant(survey_id, participant_id));

        assert_ok!(PalletSurvey::register_participant(
            RuntimeOrigin::signed(survey_owner),
            survey_id,
            participant_id
        ));

        // Test events
        let mut events = get_events();
        assert_eq!(
            events.pop(),
            Some(Event::NewParticipantRegistered {
                survey_id,
                participant_id
            })
        );

        assert!(PalletSurvey::is_participant(survey_id, participant_id));
    });
}

#[test]
fn register_participant_fails_survey_not_created() {
    new_test_ext().execute_with(|| {
        let (survey_owner, participant_id) = initialize_state();
        let survey_id: SurveyId = 0;

        assert_noop!(
            PalletSurvey::register_participant(
                RuntimeOrigin::signed(survey_owner),
                survey_id,
                participant_id
            ),
            crate::Error::<Test>::SurveyNotCreated
        );
    });
}

#[test]
fn register_participant_fails_survey_not_funded() {
    new_test_ext().execute_with(|| {
        let (survey_owner, participant_id) = initialize_state();
        let survey_id: SurveyId = 0;
        let participants_limit: ParticipantLimitType = 1000000;

        assert_ok!(PalletSurvey::create_survey(
            RuntimeOrigin::signed(survey_owner),
            survey_id,
            participants_limit
        ));

        assert_noop!(
            PalletSurvey::register_participant(
                RuntimeOrigin::signed(survey_owner),
                survey_id,
                participant_id
            ),
            crate::Error::<Test>::SurveyNotFunded
        );
    });
}

#[test]
fn register_participant_fails_participant_already_registered() {
    new_test_ext().execute_with(|| {
        let (survey_owner, participant_id) = initialize_state();
        let survey_id: SurveyId = 0;
        let participants_limit: ParticipantLimitType = 1000000;
        let fund_amount = 1000000;

        assert_ok!(PalletSurvey::create_and_fund_survey(
            RuntimeOrigin::signed(survey_owner),
            survey_id,
            participants_limit,
            fund_amount
        ));

        assert_ok!(PalletSurvey::register_participant(
            RuntimeOrigin::signed(survey_owner),
            survey_id,
            participant_id
        ));

        assert_noop!(
            PalletSurvey::register_participant(
                RuntimeOrigin::signed(survey_owner),
                survey_id,
                participant_id
            ),
            crate::Error::<Test>::ParticipantAlreadyRegistered
        );
    });
}

#[test]
fn register_participant_fails_not_owner() {
    new_test_ext().execute_with(|| {
        let (survey_owner, participant_id) = initialize_state();
        let survey_id: SurveyId = 0;
        let participants_limit: ParticipantLimitType = 1000000;
        let fund_amount = 1000000;

        assert_ok!(PalletSurvey::create_and_fund_survey(
            RuntimeOrigin::signed(survey_owner),
            survey_id,
            participants_limit,
            fund_amount
        ));

        assert_noop!(
            PalletSurvey::register_participant(
                RuntimeOrigin::signed(participant_id),
                survey_id,
                participant_id
            ),
            crate::Error::<Test>::NotOwnerOfSurvey
        );
    });
}

#[test]
fn register_participant_fails_max_number_participants_reached() {
    new_test_ext().execute_with(|| {
        let (survey_owner, participant_id) = initialize_state();
        let second_participant: u64 = 3;
        let survey_id: SurveyId = 0;
        let participants_limit: ParticipantLimitType = 1;
        let fund_amount = 1000000;

        assert_ok!(PalletSurvey::create_and_fund_survey(
            RuntimeOrigin::signed(survey_owner),
            survey_id,
            participants_limit,
            fund_amount
        ));

        assert_ok!(PalletSurvey::register_participant(
            RuntimeOrigin::signed(survey_owner),
            survey_id,
            participant_id
        ));

        assert_noop!(
            PalletSurvey::register_participant(
                RuntimeOrigin::signed(survey_owner),
                survey_id,
                second_participant
            ),
            crate::Error::<Test>::MaxNumberOfParticipantsReached
        );
    });
}

#[test]
fn register_participant_fails_survey_is_not_active() {
    new_test_ext().execute_with(|| {
        let (survey_owner, participant_id) = initialize_state();
        let survey_id: SurveyId = 0;
        let participants_limit: ParticipantLimitType = 1;
        let fund_amount = 1000000;

        assert_ok!(PalletSurvey::create_and_fund_survey(
            RuntimeOrigin::signed(survey_owner),
            survey_id,
            participants_limit,
            fund_amount
        ));

        assert_ok!(PalletSurvey::set_survey_status(
            RuntimeOrigin::signed(survey_owner),
            survey_id,
            Status::Paused,
        ));

        assert_noop!(
            PalletSurvey::register_participant(
                RuntimeOrigin::signed(survey_owner),
                survey_id,
                participant_id
            ),
            crate::Error::<Test>::SurveyIsNotActive
        );

        assert_ok!(PalletSurvey::set_survey_status(
            RuntimeOrigin::signed(survey_owner),
            survey_id,
            Status::Completed,
        ));

        assert_noop!(
            PalletSurvey::register_participant(
                RuntimeOrigin::signed(survey_owner),
                survey_id,
                participant_id
            ),
            crate::Error::<Test>::SurveyIsNotActive
        );
    });
}

// set_survey_status
fn set_survey_status_success() {
    new_test_ext().execute_with(|| {
        let (survey_owner, _participant_id) = initialize_state();
        let survey_id: SurveyId = 0;
        let participants_limit: ParticipantLimitType = 1000000;
        let fund_amount = 1000000;

        assert_ok!(PalletSurvey::create_survey(
            RuntimeOrigin::signed(survey_owner),
            survey_id,
            participants_limit,
        ));

        let survey = get_survey(survey_id);
        assert_eq!(survey.status, Status::Active);

        assert_ok!(PalletSurvey::set_survey_status(
            RuntimeOrigin::signed(survey_owner),
            survey_id,
            Status::Paused,
        ));

        let survey = get_survey(survey_id);
        assert_eq!(survey.status, Status::Paused);

        assert_ok!(PalletSurvey::set_survey_status(
            RuntimeOrigin::signed(survey_owner),
            survey_id,
            Status::Completed,
        ));

        let survey = get_survey(survey_id);
        assert_eq!(survey.status, Status::Completed);
    });
}

fn set_survey_status_fails_not_owner() {
    new_test_ext().execute_with(|| {
        let (survey_owner, participant_id) = initialize_state();
        let survey_id: SurveyId = 0;
        let participants_limit: ParticipantLimitType = 1000000;
        let fund_amount = 1000000;

        assert_ok!(PalletSurvey::create_survey(
            RuntimeOrigin::signed(survey_owner),
            survey_id,
            participants_limit,
        ));

        let survey = get_survey(survey_id);
        assert_eq!(survey.status, Status::Active);

        assert_noop!(
            PalletSurvey::set_survey_status(
                RuntimeOrigin::signed(survey_owner),
                survey_id,
                Status::Paused,
            ),
            crate::Error::<Test>::NotOwnerOfSurvey
        );
    });
}

// reward_participant
#[test]
fn reward_participant_success() {
    new_test_ext().execute_with(|| {
        let (survey_owner, participant_id) = initialize_state();
        let survey_id: SurveyId = 0;
        let participants_limit: ParticipantLimitType = 1000000;
        let fund_amount = 1000000;

        assert_ok!(PalletSurvey::create_and_fund_survey(
            RuntimeOrigin::signed(survey_owner),
            survey_id,
            participants_limit,
            fund_amount
        ));

        assert_ok!(PalletSurvey::register_participant(
            RuntimeOrigin::signed(survey_owner),
            survey_id,
            participant_id
        ));

        let balance_participant_before =
            <<Test as Config>::NativeBalance as fungible::Inspect<u64>>::balance(&participant_id);

        assert_ok!(PalletSurvey::reward_participant(
            RuntimeOrigin::signed(survey_owner),
            survey_id,
            participant_id
        ));

        let reward_amount_expected = 1u32.into();
        // Test events
        let mut events = get_events();
        assert_eq!(
            events.pop(),
            Some(Event::RewardClaimed {
                survey_id,
                participant_id,
                reward_amount: reward_amount_expected
            })
        );

        // Check that balance of participant has been updated
        let balance_participant_after =
            <<Test as Config>::NativeBalance as fungible::Inspect<u64>>::balance(&participant_id);

        assert_eq!(
            balance_participant_after,
            balance_participant_before + reward_amount_expected
        );
    });
}

#[test]
fn reward_participant_fails_survey_not_created() {
    new_test_ext().execute_with(|| {
        let (survey_owner, participant_id) = initialize_state();
        let survey_id: SurveyId = 0;

        assert_noop!(
            PalletSurvey::register_participant(
                RuntimeOrigin::signed(survey_owner),
                survey_id,
                participant_id
            ),
            crate::Error::<Test>::SurveyNotCreated
        );

        assert_noop!(
            PalletSurvey::reward_participant(
                RuntimeOrigin::signed(survey_owner),
                survey_id,
                participant_id
            ),
            crate::Error::<Test>::SurveyNotCreated
        );
    });
}

#[test]
fn reward_participant_fails_survey_not_funded() {
    new_test_ext().execute_with(|| {
        let (survey_owner, participant_id) = initialize_state();
        let survey_id: SurveyId = 0;

        let participants_limit: ParticipantLimitType = 1000000;

        assert_ok!(PalletSurvey::create_survey(
            RuntimeOrigin::signed(survey_owner),
            survey_id,
            participants_limit,
        ));

        assert_noop!(
            PalletSurvey::register_participant(
                RuntimeOrigin::signed(survey_owner),
                survey_id,
                participant_id
            ),
            crate::Error::<Test>::SurveyNotFunded
        );

        assert_noop!(
            PalletSurvey::reward_participant(
                RuntimeOrigin::signed(survey_owner),
                survey_id,
                participant_id
            ),
            crate::Error::<Test>::SurveyNotFunded
        );
    });
}

#[test]
fn reward_participant_fails_already_rewarded() {
    new_test_ext().execute_with(|| {
        let (survey_owner, participant_id) = initialize_state();
        let survey_id: SurveyId = 0;

        let participants_limit: ParticipantLimitType = 1000000;
        let fund_amount = 1000000;

        assert_ok!(PalletSurvey::create_and_fund_survey(
            RuntimeOrigin::signed(survey_owner),
            survey_id,
            participants_limit,
            fund_amount
        ));

        assert_ok!(PalletSurvey::register_participant(
            RuntimeOrigin::signed(survey_owner),
            survey_id,
            participant_id
        ));

        assert_ok!(PalletSurvey::reward_participant(
            RuntimeOrigin::signed(participant_id),
            survey_id,
            participant_id
        ));

        assert_noop!(
            PalletSurvey::reward_participant(
                RuntimeOrigin::signed(survey_owner),
                survey_id,
                participant_id
            ),
            crate::Error::<Test>::ParticipantAlreadyRewarded
        );
    });
}

#[test]
fn reward_participant_fails_participant_not_registered() {
    new_test_ext().execute_with(|| {
        let (survey_owner, participant_id) = initialize_state();
        let survey_id: SurveyId = 0;

        let participants_limit: ParticipantLimitType = 1000000;
        let fund_amount = 1000000;

        assert_ok!(PalletSurvey::create_and_fund_survey(
            RuntimeOrigin::signed(survey_owner),
            survey_id,
            participants_limit,
            fund_amount
        ));

        assert_noop!(
            PalletSurvey::reward_participant(
                RuntimeOrigin::signed(survey_owner),
                survey_id,
                participant_id
            ),
            crate::Error::<Test>::ParticipantNotRegistered
        );
    });
}

#[test]
fn reward_participant_fails_not_owner() {
    new_test_ext().execute_with(|| {
        let (survey_owner, participant_id) = initialize_state();
        let survey_id: SurveyId = 0;

        let participants_limit: ParticipantLimitType = 1000000;
        let fund_amount = 1000000;

        assert_ok!(PalletSurvey::create_and_fund_survey(
            RuntimeOrigin::signed(survey_owner),
            survey_id,
            participants_limit,
            fund_amount
        ));

        assert_ok!(PalletSurvey::register_participant(
            RuntimeOrigin::signed(survey_owner),
            survey_id,
            participant_id
        ));

        assert_noop!(
            PalletSurvey::reward_participant(
                RuntimeOrigin::signed(participant_id),
                survey_id,
                participant_id
            ),
            crate::Error::<Test>::NotOwnerOfSurvey
        );
    });
}
