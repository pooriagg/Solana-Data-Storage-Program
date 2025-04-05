use {
    solana_program_test::{
        processor,
        tokio,
        ProgramTest,
        ProgramTestContext
    },

    solana_sdk::{
        account::{
            Account as SolanaAccount,
            AccountSharedData
        }, 
        instruction::{
            AccountMeta,
            Instruction, 
            InstructionError
        }, 
        native_token::sol_to_lamports, 
        pubkey::Pubkey, 
        rent::{
            DEFAULT_EXEMPTION_THRESHOLD, 
            DEFAULT_LAMPORTS_PER_BYTE_YEAR
        }, 
        clock::{
            Epoch,
            Clock
        },
        signature::Signer, 
        signer::keypair::Keypair, 
        system_program::ID as SYSTEM_PROGRAM_ID, 
        transaction::{
            Transaction,
            TransactionError
        },
        system_instruction::SystemError
    },

    data_storage::{
        process_instruction,
        DataStorageError,
        CREATE_NEW_DATA_STORAGE_ACCOUNT_INSTRUCTION_DISCRIMINATOR,
        EDIT_DATA_STORAGE_ACCOUNT_INSTRUCTION_DISCRIMINATOR,
        CLOSE_DATA_STORAGE_ACCOUNT_INSTRUCTION_DISCRIMINATOR,
        Events
    },

    arrayref::{
        array_ref,
        array_refs
    }
};

fn setup(program_id: &Pubkey) -> ProgramTest {
    ProgramTest::new(
        "data_storage",
        *program_id,
        processor!(process_instruction)
    )
}

#[tokio::test]
async fn test_create_and_initialize_new_data_storage_account() {
    let data_storage_program_id = Pubkey::new_from_array([1; 32]);
    let mut pt = setup(&data_storage_program_id);
    
    //? Add authority account
    let authority_keypair = Keypair::new();
    let authority_account = SolanaAccount::new(
        sol_to_lamports(0.01),
        0,
        &SYSTEM_PROGRAM_ID
    );
    pt.add_account(
        authority_keypair.pubkey(),
        authority_account
    );
    //? Add authority account

    // start testing environmnet
    let mut ptc: ProgramTestContext = pt.start_with_context().await;

    ptc
        .get_new_latest_blockhash()
        .await
        .unwrap();

    // success - create a mutable data storage account
    {
        let mut data_storage_account_label: [u8; 30] = [0; 30];
        data_storage_account_label.fill(65);

        let data_storage_pda = Pubkey::find_program_address(
            &[
                b"data_storage_account",
                authority_keypair.pubkey().to_bytes().as_slice(),
                &data_storage_account_label
            ],
            &data_storage_program_id
        );

        let instruction_accounts: [AccountMeta; 4] = [
            AccountMeta::new(data_storage_pda.0, false),
            AccountMeta::new_readonly(authority_keypair.pubkey(), true),
            AccountMeta::new(ptc.payer.pubkey(), true),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false)
        ];

        let data_storage_account_data = String::from("PooriaGG..");

        let instruction_data: &[u8] = &[
            &[ CREATE_NEW_DATA_STORAGE_ACCOUNT_INSTRUCTION_DISCRIMINATOR ],
            data_storage_account_label.as_slice(),
            data_storage_account_data.as_bytes()
        ].concat();

        let instruction = Instruction {
            program_id: data_storage_program_id,
            accounts: instruction_accounts.to_vec(),
            data: instruction_data.to_vec()
        };

        let transaction = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &authority_keypair
            ],
            ptc.last_blockhash
        );

        // validate emitted event
        let simulation_result = ptc
            .banks_client
            .simulate_transaction(transaction.clone())
            .await
            .unwrap();

        let event = Events::NewDataStorageAccountCreated {
            data_storage_account: data_storage_pda.0,
            authority_account: authority_keypair.pubkey(),
            account_label: data_storage_account_label
        };
        let log_event = format!("Program log: {:?}", event);

        assert_eq!(
            simulation_result
                .simulation_details
                .unwrap()
                .logs
                .contains(&log_event),
            true,
            "Invalid emitted event!"
        );

        ptc
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap();

        let SolanaAccount { owner, data, .. } = ptc
            .banks_client
            .get_account(data_storage_pda.0)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            owner,
            data_storage_program_id,
            "Invalid data_storage_account's owner."
        );

        assert_eq!(
            data.len(),
            84,
            "Invalid data length."
        );
        
        let dsa_data = array_ref![ data, 0, 84 ];
        let (
            expected_authority,
            expected_label,
            expected_last_updated,
            _,
            expected_is_initialize,
            expected_data_length,
            expected_data
        ) = array_refs![ dsa_data, 32, 30, 8, 1, 1, 2, 10 ];

        assert_eq!(
            *expected_authority,
            authority_keypair.pubkey().to_bytes(),
            "Invalid expected authority."
        );
        assert_eq!(
            *expected_label,
            data_storage_account_label,
            "Invalid expected label."
        );
        assert_eq!(
            *expected_last_updated,
            [ 0u8; 8 ],
            "Invalid expected last_updated."
        );
        assert_eq!(
            u8::from_le_bytes(*expected_is_initialize),
            1u8,
            "Invalid expected is_initialized flag."
        );
        assert_eq!(
            u16::from_le_bytes(*expected_data_length),
            10u16,
            "Invalid expected data_length."
        );
        assert_eq!(
            String::from_utf8(expected_data.to_vec()).unwrap(),
            data_storage_account_data,
            "Invalid expected data."
        );
    };
    // success - create a mutable data storage account

    ptc
        .get_new_latest_blockhash()
        .await
        .unwrap();

    // success - create an immutable data storage account
    {
        let mut data_storage_account_label: [u8; 30] = [0; 30];
        data_storage_account_label.fill(65);

        let data_storage_pda = Pubkey::find_program_address(
            &[
                b"data_storage_account",
                SYSTEM_PROGRAM_ID.to_bytes().as_slice(),
                &data_storage_account_label
            ],
            &data_storage_program_id
        );

        let instruction_accounts: [AccountMeta; 4] = [
            AccountMeta::new(data_storage_pda.0, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            AccountMeta::new(ptc.payer.pubkey(), true),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false)
        ];

        let data_storage_account_data = String::from("PooriaGG..");

        let instruction_data: &[u8] = &[
            &[ CREATE_NEW_DATA_STORAGE_ACCOUNT_INSTRUCTION_DISCRIMINATOR ],
            data_storage_account_label.as_slice(),
            data_storage_account_data.as_bytes()
        ].concat();

        let instruction = Instruction {
            program_id: data_storage_program_id,
            accounts: instruction_accounts.to_vec(),
            data: instruction_data.to_vec()
        };

        let transaction = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[ &ptc.payer ],
            ptc.last_blockhash
        );

        ptc
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap();

        let SolanaAccount { owner, data, .. } = ptc
            .banks_client
            .get_account(data_storage_pda.0)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            owner,
            data_storage_program_id,
            "Invalid data_storage_account's owner."
        );

        assert_eq!(
            data.len(),
            84,
            "Invalid data length."
        );
        
        let dsa_data = array_ref![ data, 0, 84 ];
        let (
            expected_authority,
            expected_label,
            expected_last_updated,
            _,
            expected_is_initialize,
            expected_data_length,
            expected_data
        ) = array_refs![ dsa_data, 32, 30, 8, 1, 1, 2, 10 ];

        assert_eq!(
            *expected_authority,
            SYSTEM_PROGRAM_ID.to_bytes(),
            "Invalid expected authority."
        );
        assert_eq!(
            *expected_label,
            data_storage_account_label,
            "Invalid expected label."
        );
        assert_eq!(
            *expected_last_updated,
            [ 0u8; 8 ],
            "Invalid expected last_updated."
        );
        assert_eq!(
            u8::from_le_bytes(*expected_is_initialize),
            1u8,
            "Invalid expected is_initialized flag."
        );
        assert_eq!(
            u16::from_le_bytes(*expected_data_length),
            10u16,
            "Invalid expected data_length."
        );
        assert_eq!(
            String::from_utf8(expected_data.to_vec()).unwrap(),
            data_storage_account_data,
            "Invalid expected data."
        );
    };
    // success - create an immutable data storage account

    ptc
        .get_new_latest_blockhash()
        .await
        .unwrap();

    // faliure - account already in use
    {
        let mut data_storage_account_label: [u8; 30] = [0; 30];
        data_storage_account_label.fill(65);

        let data_storage_pda = Pubkey::find_program_address(
            &[
                b"data_storage_account",
                authority_keypair.pubkey().to_bytes().as_slice(),
                &data_storage_account_label
            ],
            &data_storage_program_id
        );

        let instruction_accounts: [AccountMeta; 4] = [
            AccountMeta::new(data_storage_pda.0, false),
            AccountMeta::new_readonly(authority_keypair.pubkey(), true),
            AccountMeta::new(ptc.payer.pubkey(), true),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false)
        ];

        let data_storage_account_data = String::from("PooriaGG..");

        let instruction_data: &[u8] = &[
            &[ CREATE_NEW_DATA_STORAGE_ACCOUNT_INSTRUCTION_DISCRIMINATOR ],
            data_storage_account_label.as_slice(),
            data_storage_account_data.as_bytes()
        ].concat();

        let instruction = Instruction {
            program_id: data_storage_program_id,
            accounts: instruction_accounts.to_vec(),
            data: instruction_data.to_vec()
        };

        let transaction = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &authority_keypair
            ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    SystemError::AccountAlreadyInUse as u32
                )
            )
        );
    }
    // faliure - account already in use

    ptc
        .get_new_latest_blockhash()
        .await
        .unwrap();

    // faliure - invalid label length
    {
        let mut data_storage_account_label: [u8; 20] = [0; 20];
        data_storage_account_label.fill(97);

        let data_storage_pda = Pubkey::find_program_address(
            &[
                b"data_storage_account",
                authority_keypair.pubkey().to_bytes().as_slice(),
                &data_storage_account_label
            ],
            &data_storage_program_id
        );

        let instruction_accounts: [AccountMeta; 4] = [
            AccountMeta::new(data_storage_pda.0, false),
            AccountMeta::new_readonly(authority_keypair.pubkey(), true),
            AccountMeta::new(ptc.payer.pubkey(), true),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false)
        ];

        let data_storage_account_data = String::from("SOL");

        let instruction_data: &[u8] = &[
            &[ CREATE_NEW_DATA_STORAGE_ACCOUNT_INSTRUCTION_DISCRIMINATOR ],
            data_storage_account_label.as_slice(),
            data_storage_account_data.as_bytes()
        ].concat();

        let instruction = Instruction {
            program_id: data_storage_program_id,
            accounts: instruction_accounts.to_vec(),
            data: instruction_data.to_vec()
        };

        let transaction = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &authority_keypair
            ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    DataStorageError::InvalidData as u32
                )
            )
        );
    }
    // faliure - invalid label length

    ptc
        .get_new_latest_blockhash()
        .await
        .unwrap();

    // faliure - invalid seeds OR failed to find program address
    {
        let mut data_storage_account_label: [u8; 30] = [0; 30];
        data_storage_account_label.fill(100);

        let data_storage_pda = Pubkey::find_program_address(
            &[
                b"data_storage_account",
                authority_keypair.pubkey().to_bytes().as_slice(),
                &data_storage_account_label
            ],
            &data_storage_program_id
        );

        let instruction_accounts: [AccountMeta; 4] = [
            AccountMeta::new(data_storage_pda.0, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            AccountMeta::new(ptc.payer.pubkey(), true),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false)
        ];

        let data_storage_account_data = String::from("SOL");

        let instruction_data: &[u8] = &[
            &[ CREATE_NEW_DATA_STORAGE_ACCOUNT_INSTRUCTION_DISCRIMINATOR ],
            data_storage_account_label.as_slice(),
            data_storage_account_data.as_bytes()
        ].concat();

        let instruction = Instruction {
            program_id: data_storage_program_id,
            accounts: instruction_accounts.to_vec(),
            data: instruction_data.to_vec()
        };

        let transaction = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some(&ptc.payer.pubkey()),
            &[ &ptc.payer ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap();

        if !(
            error == TransactionError::InstructionError(0, InstructionError::InvalidSeeds) ||
            error == TransactionError::InstructionError(0, InstructionError::Custom(DataStorageError::FailedToFindProgramAddress as u32))
        ) {
            panic!("Invalid error!");
        };
    }
    // faliure - invalid seeds OR failed to find program address
}

#[tokio::test]
async fn test_edit_data_storage_account() {
    let data_storage_program_id = Pubkey::new_from_array([1; 32]);
    let mut pt = setup(&data_storage_program_id);

    //? add authority account
    let authority_keypair = Keypair::new();
    pt.add_account(
        authority_keypair.pubkey(),
        SolanaAccount::new(
            sol_to_lamports(1.0),
            0,
            &SYSTEM_PROGRAM_ID
        )
    );
    //? add authority account

    let mut ptc = pt.start_with_context().await;

    // success - new len == old len
    {
        //? add data storage account
        let mut data_storage_account_label: [u8; 30] = [0; 30];
        data_storage_account_label.fill(100);

        let (
            dsa_addr,
            dsa_bump
        ) = Pubkey::find_program_address(
            &[
                b"data_storage_account",
                authority_keypair.pubkey().to_bytes().as_slice(),
                &data_storage_account_label
            ],
            &data_storage_program_id
        );

        let account_data = vec![
            authority_keypair
                .pubkey()
                .to_bytes()
                .to_vec(),
            data_storage_account_label.to_vec(),
            i64::to_le_bytes(0).to_vec(),
            vec![ dsa_bump ],
            vec![ true as u8 ],
            u16::to_le_bytes(6).to_vec(),
            (b"Solana").to_vec()
        ].into_iter().flatten().collect::<Vec<_>>();
        let account_data_len = account_data.len();

        let account_lamport_balance = sol_to_lamports(0.01);

        ptc.set_account(
            &dsa_addr,
            &AccountSharedData::from(
                SolanaAccount {
                    data: account_data,
                    owner: data_storage_program_id,
                    lamports: account_lamport_balance,
                    rent_epoch: Epoch::default(),
                    executable: false
                }
            )
        );
        //? add data storage account

        let current_time = 300_i64;
        ptc
            .set_sysvar::<Clock>(
                &Clock {
                    unix_timestamp: current_time,
                    ..Clock::default()
                }
            );

        let new_data = "Pooria";
        let instruction_data = &[
            &[ EDIT_DATA_STORAGE_ACCOUNT_INSTRUCTION_DISCRIMINATOR ],
            new_data.as_bytes()
        ].concat();

        let instruction_accounts = vec![
            AccountMeta::new(dsa_addr, false),
            AccountMeta::new_readonly(authority_keypair.pubkey(), true)
        ];

        let instruction = Instruction {
            program_id: data_storage_program_id,
            accounts: instruction_accounts,
            data: instruction_data.to_vec()
        };

        let transaction = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some( &ptc.payer.pubkey() ),
            &[
                &ptc.payer,
                &authority_keypair
            ],
            ptc.last_blockhash
        );

        // validate emitted event
        let simulation_result = ptc
            .banks_client
            .simulate_transaction(transaction.clone())
            .await
            .unwrap();

        let event = Events::DataStorageAccountEdited {
            data_storage_account: dsa_addr,
            authority_account: authority_keypair.pubkey(),
            old_data_len: 6,
            new_data_len: new_data.len()
        };
        let log_event = format!("Program log: {:?}", event);

        assert_eq!(
            simulation_result
                .simulation_details
                .unwrap()
                .logs
                .contains(&log_event),
            true,
            "Invalid emitted event!"
        );

        ptc
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap();

        let SolanaAccount { data, lamports, .. } = ptc
            .banks_client
            .get_account(dsa_addr)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            account_lamport_balance,
            lamports,
            "Invalid PDA lamport balance."
        );

        assert_eq!(
            data.len(),
            account_data_len,
            "Invalid data len."
        );

        let dsa_data = array_ref![ data, 0, 80 ];
        let (
            _,
            _,
            expected_last_updated,
            _,
            _,
            expected_data_len,
            expected_data
        ) = array_refs![ dsa_data, 32, 30, 8, 1, 1, 2, 6 ];

        assert_eq!(
            expected_data_len,
            &u16::to_le_bytes(new_data.len() as u16),
            "Ivnalid data len."
        );

        assert_eq!(
            expected_data.as_slice(),
            new_data.as_bytes(),
            "Invalid new data."
        );

        assert_eq!(
            expected_last_updated,
            &current_time.to_le_bytes(),
            "Invalid time."
        );
    }
    // success - new len == old len

    ptc
        .get_new_latest_blockhash()
        .await
        .unwrap();

    // success - new len < old len
    {
        //? add data storage account
        let mut data_storage_account_label: [u8; 30] = [0; 30];
        data_storage_account_label.fill(90);

        let (
            dsa_addr,
            dsa_bump
        ) = Pubkey::find_program_address(
            &[
                b"data_storage_account",
                authority_keypair.pubkey().to_bytes().as_slice(),
                &data_storage_account_label
            ],
            &data_storage_program_id
        );

        let old_data = "Solana";
        let account_data = vec![
            authority_keypair
                .pubkey()
                .to_bytes()
                .to_vec(),
            data_storage_account_label.to_vec(),
            i64::to_le_bytes(0).to_vec(),
            vec![ dsa_bump ],
            vec![ true as u8 ],
            u16::to_le_bytes(old_data.len() as u16).to_vec(),
            old_data
                .as_bytes()
                .to_vec()
        ].into_iter().flatten().collect::<Vec<_>>();
        let account_data_len = account_data.len();

        let dsa_account_lamport_balance = sol_to_lamports(0.01);

        ptc.set_account(
            &dsa_addr,
            &AccountSharedData::from(
                SolanaAccount {
                    data: account_data,
                    owner: data_storage_program_id,
                    lamports: dsa_account_lamport_balance,
                    rent_epoch: Epoch::default(),
                    executable: false
                }
            )
        );
        //? add data storage account

        let current_time = 450_i64;
        ptc
            .set_sysvar::<Clock>(
                &Clock {
                    unix_timestamp: current_time,
                    ..Clock::default()
                }
            );

        let new_data = "SOL";
        let instruction_data = &[
            &[ EDIT_DATA_STORAGE_ACCOUNT_INSTRUCTION_DISCRIMINATOR ],
            new_data.as_bytes()
        ].concat();

        let instruction_accounts = vec![
            AccountMeta::new(dsa_addr, false),
            AccountMeta::new_readonly(authority_keypair.pubkey(), true),
            AccountMeta::new(authority_keypair.pubkey(), false)
        ];

        let instruction = Instruction {
            program_id: data_storage_program_id,
            accounts: instruction_accounts,
            data: instruction_data.to_vec()
        };

        let transaction = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some( &ptc.payer.pubkey() ),
            &[
                &ptc.payer,
                &authority_keypair
            ],
            ptc.last_blockhash
        );

        let rent_receiver_before_tx_lamport_balance = ptc
            .banks_client
            .get_balance(authority_keypair.pubkey())
            .await
            .unwrap();

        // validate emitted event
        let simulation_result = ptc
            .banks_client
            .simulate_transaction(transaction.clone())
            .await
            .unwrap();

        let event = Events::DataStorageAccountEdited {
            data_storage_account: dsa_addr,
            authority_account: authority_keypair.pubkey(),
            old_data_len: old_data.len(),
            new_data_len: new_data.len()
        };
        let log_event = format!("Program log: {:?}", event);

        assert_eq!(
            simulation_result
                .simulation_details
                .unwrap()
                .logs
                .contains(&log_event),
            true,
            "Invalid emitted event!"
        );

        ptc
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap();

        let SolanaAccount { data, lamports: dsa_after_tx_lamport_balance, .. } = ptc
            .banks_client
            .get_account(dsa_addr)
            .await
            .unwrap()
            .unwrap();

        let rent_receiver_after_tx_lamport_balance = ptc
            .banks_client
            .get_balance(authority_keypair.pubkey())
            .await
            .unwrap();

        let extra_rent_exempt_lamports = (
            (old_data.len() - new_data.len()) as u64 * DEFAULT_LAMPORTS_PER_BYTE_YEAR
        ) * DEFAULT_EXEMPTION_THRESHOLD as u64;

        assert_eq!(
            dsa_account_lamport_balance - extra_rent_exempt_lamports,
            dsa_after_tx_lamport_balance,
            "Invalid data-storage-account's lamport balance."
        );

        assert_eq!(
            rent_receiver_before_tx_lamport_balance + extra_rent_exempt_lamports,
            rent_receiver_after_tx_lamport_balance,
            "Invalid rent_receiver_account's lamport balance."
        );

        let new_account_data_len = account_data_len - ( old_data.len() - new_data.len() ) ;
        assert_eq!(
            new_account_data_len,
            data.len(),
            "Invalid data-storage-account's data len."
        );

        let dsa_data = array_ref![ data, 0, 77 ];
        let (
            _,
            _,
            expected_last_updated,
            _,
            _,
            expected_data_len,
            expected_data
        ) = array_refs![ dsa_data, 32, 30, 8, 1, 1, 2, 3 ];

        assert_eq!(
            expected_data_len,
            &u16::to_le_bytes(new_data.len() as u16),
            "Invalid data len."
        );

        assert_eq!(
            expected_data.as_slice(),
            new_data.as_bytes(),
            "Invalid new data."
        );

        assert_eq!(
            expected_last_updated,
            &i64::to_le_bytes(current_time),
            "Invalid last-updated-time."
        );
    }
    // success - new len < old len

    ptc
        .get_new_latest_blockhash()
        .await
        .unwrap();

    // success - new len > old len
    {
        //? add data storage account
        let mut data_storage_account_label: [u8; 30] = [0; 30];
        data_storage_account_label.fill(68);

        let (
            dsa_addr,
            dsa_bump
        ) = Pubkey::find_program_address(
            &[
                b"data_storage_account",
                authority_keypair.pubkey().to_bytes().as_slice(),
                &data_storage_account_label
            ],
            &data_storage_program_id
        );

        let old_data = "Solana";
        let account_data = vec![
            authority_keypair
                .pubkey()
                .to_bytes()
                .to_vec(),
            data_storage_account_label.to_vec(),
            i64::to_le_bytes(0).to_vec(),
            vec![ dsa_bump ],
            vec![ true as u8 ],
            u16::to_le_bytes(old_data.len() as u16).to_vec(),
            old_data
                .as_bytes()
                .to_vec()
        ].into_iter().flatten().collect::<Vec<_>>();
        let account_data_len = account_data.len();

        let dsa_account_lamport_balance = sol_to_lamports(0.01);

        ptc.set_account(
            &dsa_addr,
            &AccountSharedData::from(
                SolanaAccount {
                    data: account_data,
                    owner: data_storage_program_id,
                    lamports: dsa_account_lamport_balance,
                    rent_epoch: Epoch::default(),
                    executable: false
                }
            )
        );
        //? add data storage account

        let current_time = 550_i64;
        ptc
            .set_sysvar::<Clock>(
                &Clock {
                    unix_timestamp: current_time,
                    ..Clock::default()
                }
            );

        let new_data = "PooriaGG!";
        let instruction_data = &[
            &[ EDIT_DATA_STORAGE_ACCOUNT_INSTRUCTION_DISCRIMINATOR ],
            new_data.as_bytes()
        ].concat();

        let instruction_accounts = vec![
            AccountMeta::new(dsa_addr, false),
            AccountMeta::new_readonly(authority_keypair.pubkey(), true),
            AccountMeta::new(authority_keypair.pubkey(), false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false)
        ];

        let instruction = Instruction {
            program_id: data_storage_program_id,
            accounts: instruction_accounts,
            data: instruction_data.to_vec()
        };

        let transaction = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some( &ptc.payer.pubkey() ),
            &[
                &ptc.payer,
                &authority_keypair
            ],
            ptc.last_blockhash
        );

        let funding_account_before_tx_lamport_balance = ptc
            .banks_client
            .get_balance(authority_keypair.pubkey())
            .await
            .unwrap();

        // validate emitted event
        let simulation_result = ptc
            .banks_client
            .simulate_transaction(transaction.clone())
            .await
            .unwrap();

        let event = Events::DataStorageAccountEdited {
            data_storage_account: dsa_addr,
            authority_account: authority_keypair.pubkey(),
            old_data_len: old_data.len(),
            new_data_len: new_data.len()
        };
        let log_event = format!("Program log: {:?}", event);

        assert_eq!(
            simulation_result
                .simulation_details
                .unwrap()
                .logs
                .contains(&log_event),
            true,
            "Invalid emitted event!"
        );

        ptc
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap();

        let SolanaAccount { data, lamports: dsa_after_tx_lamport_balance, .. } = ptc
            .banks_client
            .get_account(dsa_addr)
            .await
            .unwrap()
            .unwrap();

        let funding_account_after_tx_lamport_balance = ptc
            .banks_client
            .get_balance(authority_keypair.pubkey())
            .await
            .unwrap();

        let extra_rent_exempt_lamports = (
            (new_data.len() - old_data.len()) as u64 * DEFAULT_LAMPORTS_PER_BYTE_YEAR
        ) * DEFAULT_EXEMPTION_THRESHOLD as u64;

        assert_eq!(
            dsa_account_lamport_balance + extra_rent_exempt_lamports,
            dsa_after_tx_lamport_balance,
            "Invalid data-storage-account's lamport balance."
        );

        assert_eq!(
            funding_account_before_tx_lamport_balance - extra_rent_exempt_lamports,
            funding_account_after_tx_lamport_balance,
            "Invalid funding_account's lamport balance."
        );

        let new_account_data_len = account_data_len + ( new_data.len() - old_data.len() );
        assert_eq!(
            new_account_data_len,
            data.len(),
            "Invalid data-storage-account's data len."
        );

        let dsa_data = array_ref![ data, 0, 83 ];
        let (
            _,
            _,
            expected_last_updated,
            _,
            _,
            expected_data_len,
            expected_data
        ) = array_refs![ dsa_data, 32, 30, 8, 1, 1, 2, 9 ];

        assert_eq!(
            expected_data_len,
            &u16::to_le_bytes(new_data.len() as u16),
            "Invalid data len."
        );

        assert_eq!(
            expected_data.as_slice(),
            new_data.as_bytes(),
            "Invalid new data."
        );

        assert_eq!(
            expected_last_updated,
            &i64::to_le_bytes(current_time),
            "Invalid last-updated-time."
        );
    }  
    // success - new len > old len

    ptc
        .get_new_latest_blockhash()
        .await
        .unwrap();

    // failure - immutable data storage account
    {
        //? add data storage account
        let mut data_storage_account_label: [u8; 30] = [0; 30];
        data_storage_account_label.fill(82);

        let (
            dsa_addr,
            dsa_bump
        ) = Pubkey::find_program_address(
            &[
                b"data_storage_account",
                authority_keypair.pubkey().to_bytes().as_slice(),
                &data_storage_account_label
            ],
            &data_storage_program_id
        );

        let old_data = "Solana";
        let account_data = vec![
            SYSTEM_PROGRAM_ID
                .to_bytes()
                .to_vec(),
            data_storage_account_label.to_vec(),
            i64::to_le_bytes(0).to_vec(),
            vec![ dsa_bump ],
            vec![ true as u8 ],
            u16::to_le_bytes(old_data.len() as u16).to_vec(),
            old_data
                .as_bytes()
                .to_vec()
        ].into_iter().flatten().collect::<Vec<_>>();

        let dsa_account_lamport_balance = sol_to_lamports(0.01);

        ptc.set_account(
            &dsa_addr,
            &AccountSharedData::from(
                SolanaAccount {
                    data: account_data,
                    owner: data_storage_program_id,
                    lamports: dsa_account_lamport_balance,
                    rent_epoch: Epoch::default(),
                    executable: false
                }
            )
        );
        //? add data storage account

        let new_data = "Pooria";
        let instruction_data = &[
            &[ EDIT_DATA_STORAGE_ACCOUNT_INSTRUCTION_DISCRIMINATOR ],
            new_data.as_bytes()
        ].concat();

        let instruction_accounts = vec![
            AccountMeta::new(dsa_addr, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ];

        let instruction = Instruction {
            program_id: data_storage_program_id,
            accounts: instruction_accounts,
            data: instruction_data.to_vec()
        };

        let transaction = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some( &ptc.payer.pubkey() ),
            &[ &ptc.payer ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    DataStorageError::ImmutableDataStorage as u32
                )
            )
        );
    }
    // failure - immutable data storage account

    ptc
        .get_new_latest_blockhash()
        .await
        .unwrap();

    // failure - authority account is not signer
    {
        //? add data storage account
        let mut data_storage_account_label: [u8; 30] = [0; 30];
        data_storage_account_label.fill(82);

        let (
            dsa_addr,
            dsa_bump
        ) = Pubkey::find_program_address(
            &[
                b"data_storage_account",
                authority_keypair.pubkey().to_bytes().as_slice(),
                &data_storage_account_label
            ],
            &data_storage_program_id
        );

        let old_data = "Solana";
        let account_data = vec![
            authority_keypair
                .pubkey()
                .to_bytes()
                .to_vec(),
            data_storage_account_label.to_vec(),
            i64::to_le_bytes(0).to_vec(),
            vec![ dsa_bump ],
            vec![ true as u8 ],
            u16::to_le_bytes(old_data.len() as u16).to_vec(),
            old_data
                .as_bytes()
                .to_vec()
        ].into_iter().flatten().collect::<Vec<_>>();

        let dsa_account_lamport_balance = sol_to_lamports(0.01);

        ptc.set_account(
            &dsa_addr,
            &AccountSharedData::from(
                SolanaAccount {
                    data: account_data,
                    owner: data_storage_program_id,
                    lamports: dsa_account_lamport_balance,
                    rent_epoch: Epoch::default(),
                    executable: false
                }
            )
        );
        //? add data storage account

        let new_data = "Pooria";
        let instruction_data = &[
            &[ EDIT_DATA_STORAGE_ACCOUNT_INSTRUCTION_DISCRIMINATOR ],
            new_data.as_bytes()
        ].concat();

        let instruction_accounts = vec![
            AccountMeta::new(dsa_addr, false),
            AccountMeta::new_readonly(authority_keypair.pubkey(), false),
        ];

        let instruction = Instruction {
            program_id: data_storage_program_id,
            accounts: instruction_accounts,
            data: instruction_data.to_vec()
        };

        let transaction = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some( &ptc.payer.pubkey() ),
            &[ &ptc.payer ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::MissingRequiredSignature
            )
        );
    }
    // failure - authority account is not signer

    ptc
        .get_new_latest_blockhash()
        .await
        .unwrap();

    // failure - invalid data-storage-account's program owner
    {
        //? add data storage account
        let mut data_storage_account_label: [u8; 30] = [0; 30];
        data_storage_account_label.fill(82);

        let (
            dsa_addr,
            dsa_bump
        ) = Pubkey::find_program_address(
            &[
                b"data_storage_account",
                authority_keypair.pubkey().to_bytes().as_slice(),
                &data_storage_account_label
            ],
            &data_storage_program_id
        );

        let old_data = "Solana";
        let account_data = vec![
            authority_keypair
                .pubkey()
                .to_bytes()
                .to_vec(),
            data_storage_account_label.to_vec(),
            i64::to_le_bytes(0).to_vec(),
            vec![ dsa_bump ],
            vec![ true as u8 ],
            u16::to_le_bytes(old_data.len() as u16).to_vec(),
            old_data
                .as_bytes()
                .to_vec()
        ].into_iter().flatten().collect::<Vec<_>>();

        let dsa_account_lamport_balance = sol_to_lamports(0.01);

        ptc.set_account(
            &dsa_addr,
            &AccountSharedData::from(
                SolanaAccount {
                    data: account_data,
                    owner: SYSTEM_PROGRAM_ID,
                    lamports: dsa_account_lamport_balance,
                    rent_epoch: Epoch::default(),
                    executable: false
                }
            )
        );
        //? add data storage account

        let new_data = "Pooria";
        let instruction_data = &[
            &[ EDIT_DATA_STORAGE_ACCOUNT_INSTRUCTION_DISCRIMINATOR ],
            new_data.as_bytes()
        ].concat();

        let instruction_accounts = vec![
            AccountMeta::new(dsa_addr, false),
            AccountMeta::new_readonly(authority_keypair.pubkey(), true),
        ];

        let instruction = Instruction {
            program_id: data_storage_program_id,
            accounts: instruction_accounts,
            data: instruction_data.to_vec()
        };

        let transaction = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some( &ptc.payer.pubkey() ),
            &[
                &ptc.payer,
                &authority_keypair
            ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::InvalidAccountOwner
            )
        );
    }
    // failure - invalid data-storage-account's program owner

    ptc
        .get_new_latest_blockhash()
        .await
        .unwrap();

    // failure - account is not initialized
    {
        //? add data storage account
        let mut data_storage_account_label: [u8; 30] = [0; 30];
        data_storage_account_label.fill(82);

        let (
            dsa_addr,
            dsa_bump
        ) = Pubkey::find_program_address(
            &[
                b"data_storage_account",
                authority_keypair.pubkey().to_bytes().as_slice(),
                &data_storage_account_label
            ],
            &data_storage_program_id
        );

        let old_data = "Solana";
        let account_data = vec![
            authority_keypair
                .pubkey()
                .to_bytes()
                .to_vec(),
            data_storage_account_label.to_vec(),
            i64::to_le_bytes(0).to_vec(),
            vec![ dsa_bump ],
            vec![ false as u8 ],
            u16::to_le_bytes(old_data.len() as u16).to_vec(),
            old_data
                .as_bytes()
                .to_vec()
        ].into_iter().flatten().collect::<Vec<_>>();

        let dsa_account_lamport_balance = sol_to_lamports(0.01);

        ptc.set_account(
            &dsa_addr,
            &AccountSharedData::from(
                SolanaAccount {
                    data: account_data,
                    owner: data_storage_program_id,
                    lamports: dsa_account_lamport_balance,
                    rent_epoch: Epoch::default(),
                    executable: false
                }
            )
        );
        //? add data storage account

        let new_data = "Pooria";
        let instruction_data = &[
            &[ EDIT_DATA_STORAGE_ACCOUNT_INSTRUCTION_DISCRIMINATOR ],
            new_data.as_bytes()
        ].concat();

        let instruction_accounts = vec![
            AccountMeta::new(dsa_addr, false),
            AccountMeta::new_readonly(authority_keypair.pubkey(), true),
        ];

        let instruction = Instruction {
            program_id: data_storage_program_id,
            accounts: instruction_accounts,
            data: instruction_data.to_vec()
        };

        let transaction = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some( &ptc.payer.pubkey() ),
            &[
                &ptc.payer,
                &authority_keypair
            ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::UninitializedAccount
            )
        );
    }
    // failure - account is not initialized

    ptc
        .get_new_latest_blockhash()
        .await
        .unwrap();

    // failure - invalid authority
    {
        //? add data storage account
        let mut data_storage_account_label: [u8; 30] = [0; 30];
        data_storage_account_label.fill(82);

        let unknown_wallet = Keypair::new();

        let (
            dsa_addr,
            dsa_bump
        ) = Pubkey::find_program_address(
            &[
                b"data_storage_account",
                authority_keypair.pubkey().to_bytes().as_slice(),
                &data_storage_account_label
            ],
            &data_storage_program_id
        );

        let old_data = "Solana";
        let account_data = vec![
            authority_keypair
                .pubkey()
                .to_bytes()
                .to_vec(),
            data_storage_account_label.to_vec(),
            i64::to_le_bytes(0).to_vec(),
            vec![ dsa_bump ],
            vec![ true as u8 ],
            u16::to_le_bytes(old_data.len() as u16).to_vec(),
            old_data
                .as_bytes()
                .to_vec()
        ].into_iter().flatten().collect::<Vec<_>>();

        let dsa_account_lamport_balance = sol_to_lamports(0.01);

        ptc.set_account(
            &dsa_addr,
            &AccountSharedData::from(
                SolanaAccount {
                    data: account_data,
                    owner: data_storage_program_id,
                    lamports: dsa_account_lamport_balance,
                    rent_epoch: Epoch::default(),
                    executable: false
                }
            )
        );
        //? add data storage account

        let new_data = "Pooria";
        let instruction_data = &[
            &[ EDIT_DATA_STORAGE_ACCOUNT_INSTRUCTION_DISCRIMINATOR ],
            new_data.as_bytes()
        ].concat();

        let instruction_accounts = vec![
            AccountMeta::new(dsa_addr, false),
            AccountMeta::new_readonly(unknown_wallet.pubkey(), true),
        ];

        let instruction = Instruction {
            program_id: data_storage_program_id,
            accounts: instruction_accounts,
            data: instruction_data.to_vec()
        };

        let transaction = Transaction::new_signed_with_payer(
            &[ instruction ],
            Some( &ptc.payer.pubkey() ),
            &[
                &ptc.payer,
                &unknown_wallet
            ],
            ptc.last_blockhash
        );

        let error = ptc
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::IncorrectAuthority
            )
        );
    }
    // failure - invalid authority

    // faliure - invalid seeds OR failed to find program address
    {
        //? Impossible to get this error
    }
    // faliure - invalid seeds OR failed to find program address
}

#[tokio::test]
async fn test_close_data_storage_account() {
    let data_storage_program_id = Pubkey::new_from_array([1; 32]);
    let mut pt = setup(&data_storage_program_id);
    
    //? add authority account
    let authority_keypair = Keypair::new();
    pt.add_account(
        authority_keypair.pubkey(),
        SolanaAccount::new(
            sol_to_lamports(1.0),
            0,
            &SYSTEM_PROGRAM_ID
        )
    );
    //? add authority account

    let mut ptc = pt.start_with_context().await;

    // success
    {
        //? add data storage account
        let mut data_storage_account_label: [u8; 30] = [0; 30];
        data_storage_account_label.fill(100);

        let (
            dsa_addr,
            dsa_bump
        ) = Pubkey::find_program_address(
            &[
                b"data_storage_account",
                authority_keypair.pubkey().to_bytes().as_slice(),
                &data_storage_account_label
            ],
            &data_storage_program_id
        );

        let account_data = vec![
            authority_keypair
                .pubkey()
                .to_bytes()
                .to_vec(),
            data_storage_account_label.to_vec(),
            i64::to_le_bytes(0).to_vec(),
            vec![ dsa_bump ],
            vec![ true as u8 ],
            u16::to_le_bytes(6).to_vec(),
            (b"Solana").to_vec()
        ].into_iter().flatten().collect::<Vec<_>>();

        let account_lamport_balance = sol_to_lamports(0.01);

        ptc.set_account(
            &dsa_addr,
            &AccountSharedData::from(
                SolanaAccount {
                    data: account_data,
                    owner: data_storage_program_id,
                    lamports: account_lamport_balance,
                    rent_epoch: Epoch::default(),
                    executable: false
                }
            )
        );
        //? add data storage account

        let rent_exempt_receiver_before_tx_balance = ptc
            .banks_client
            .get_balance(authority_keypair.pubkey())
            .await
            .unwrap();

        let instruction_accounts: Vec<AccountMeta> = vec![
            AccountMeta::new(dsa_addr, false),
            AccountMeta::new_readonly(authority_keypair.pubkey(), true),
            AccountMeta::new(authority_keypair.pubkey(), false)
        ];

        let instruction_data: &[u8] = &[ CLOSE_DATA_STORAGE_ACCOUNT_INSTRUCTION_DISCRIMINATOR ];

        let instuction: Instruction = Instruction {
            program_id: data_storage_program_id,
            accounts: instruction_accounts,
            data: instruction_data.to_vec()
        };

        let transaction = Transaction::new_signed_with_payer(
            &[ instuction ],
            Some(&ptc.payer.pubkey()),
            &[
                &ptc.payer,
                &authority_keypair
            ],
            ptc.last_blockhash
        );

        // validate emitted event
        let simulation_result = ptc
            .banks_client
            .simulate_transaction(transaction.clone())
            .await
            .unwrap();
    
        let event = Events::DataStorageAccountClosed {
            data_storage_account: dsa_addr,
            authority_account: authority_keypair.pubkey()
        };
        let log_event = format!("Program log: {:?}", event);
    
        assert_eq!(
            simulation_result
                .simulation_details
                .unwrap()
                .logs
                .contains(&log_event),
            true,
            "Invalid emitted event!"
        );
    
        ptc
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap();

        let rent_exempt_receiver_after_tx_balance = ptc
            .banks_client
            .get_balance(authority_keypair.pubkey())
            .await
            .unwrap();

        assert_eq!(
            rent_exempt_receiver_before_tx_balance + sol_to_lamports(0.01),
            rent_exempt_receiver_after_tx_balance,
            "Invalid rent_exempt_receiver lamport balance."
        );

        let dsa_account_info = ptc
            .banks_client
            .get_account(dsa_addr)
            .await
            .unwrap();

        if let Some(_) = dsa_account_info {
            panic!("Account must be closed so far !");
        };
    }
    // success

    // failure - Revival Attack
    {
        //? add data storage account
        let mut data_storage_account_label: [u8; 30] = [0; 30];
        data_storage_account_label.fill(65);

        let (
            dsa_addr,
            dsa_bump
        ) = Pubkey::find_program_address(
            &[
                b"data_storage_account",
                authority_keypair.pubkey().to_bytes().as_slice(),
                &data_storage_account_label
            ],
            &data_storage_program_id
        );

        let account_data = vec![
            authority_keypair
                .pubkey()
                .to_bytes()
                .to_vec(),
            data_storage_account_label.to_vec(),
            i64::to_le_bytes(0).to_vec(),
            vec![ dsa_bump ],
            vec![ true as u8 ],
            u16::to_le_bytes(6).to_vec(),
            (b"Solana").to_vec()
        ].into_iter().flatten().collect::<Vec<_>>();

        let account_lamport_balance = sol_to_lamports(0.01);

        ptc.set_account(
            &dsa_addr,
            &AccountSharedData::from(
                SolanaAccount {
                    data: account_data,
                    owner: data_storage_program_id,
                    lamports: account_lamport_balance,
                    rent_epoch: Epoch::default(),
                    executable: false
                }
            )
        );
        //? add data storage account

        // instruction close account
        let instruction_accounts_1: Vec<AccountMeta> = vec![
            AccountMeta::new(dsa_addr, false),
            AccountMeta::new_readonly(authority_keypair.pubkey(), true),
            AccountMeta::new(authority_keypair.pubkey(), false)
        ];
    
        let instruction_data_1: &[u8] = &[ CLOSE_DATA_STORAGE_ACCOUNT_INSTRUCTION_DISCRIMINATOR ];
    
        let instuction_close_account: Instruction = Instruction {
            program_id: data_storage_program_id,
            accounts: instruction_accounts_1,
            data: instruction_data_1.to_vec()
        };

        // invoke instruction create new account
        {
            // instruction create new account
            let instruction_accounts_2: [AccountMeta; 4] = [
                AccountMeta::new(dsa_addr, false),
                AccountMeta::new_readonly(authority_keypair.pubkey(), true),
                AccountMeta::new(ptc.payer.pubkey(), true),
                AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false)
            ];
            let data_storage_account_data = String::from("PooriaGG..");
        
            let instruction_data_2: &[u8] = &[
                &[ CREATE_NEW_DATA_STORAGE_ACCOUNT_INSTRUCTION_DISCRIMINATOR ],
                data_storage_account_label.as_slice(),
                data_storage_account_data.as_bytes()
            ].concat();
        
            let instruction_create_new_account = Instruction {
                program_id: data_storage_program_id,
                accounts: instruction_accounts_2.to_vec(),
                data: instruction_data_2.to_vec()
            };

            let transaction = Transaction::new_signed_with_payer(
                &[
                    instuction_close_account.clone(),
                    instruction_create_new_account
                ],
                Some(&ptc.payer.pubkey()),
                &[
                    &ptc.payer,
                    &authority_keypair
                ],
                ptc.last_blockhash
            );

            let error = ptc
                .banks_client
                .process_transaction(transaction)
                .await
                .unwrap_err()
                .unwrap();

            assert_eq!(
                error,
                TransactionError::InstructionError(
                    1,
                    SystemError::AccountAlreadyInUse.into()
                )
            );
        }
        // invoke instruction create new account

        // invoke instruction edit account
        {
            // instruction edit account
            let new_data = "Pooria";
            let instruction_data_2 = &[
                &[ EDIT_DATA_STORAGE_ACCOUNT_INSTRUCTION_DISCRIMINATOR ],
                new_data.as_bytes()
            ].concat();
    
            let instruction_accounts_2 = vec![
                AccountMeta::new(dsa_addr, false),
                AccountMeta::new_readonly(authority_keypair.pubkey(), true)
            ];
    
            let instruction_edit_account = Instruction {
                program_id: data_storage_program_id,
                accounts: instruction_accounts_2,
                data: instruction_data_2.to_vec()
            };

            let transaction = Transaction::new_signed_with_payer(
                &[
                    instuction_close_account,
                    instruction_edit_account
                ],
                Some(&ptc.payer.pubkey()),
                &[
                    &ptc.payer,
                    &authority_keypair
                ],
                ptc.last_blockhash
            );

            let error = ptc
                .banks_client
                .process_transaction(transaction)
                .await
                .unwrap_err()
                .unwrap();

            assert_eq!(
                error,
                TransactionError::InstructionError(
                    1,
                    InstructionError::UninitializedAccount
                )
            );
        }
        // invoke instruction edit account
    }
    // failure - Revival Attack
}